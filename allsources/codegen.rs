use rand::prelude::*; // 0.8.5
use std::fmt;
use std::collections::HashMap;
use tokenizer::*;
use cli::*;
use symboltable::*;
use opcode::{Opcode,OpcodeMode};
use machineinstruction::*;
use codeframe::*;
use structmodel::*;
use macrolib::*;


//	When an expression includes a collection literal, either an
//	array or a dictionary, we want to know because the opcode
//	mode depends on it
enum CollectionContext{
	Array,
	Dict,
}
impl fmt::Display for CollectionContext{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    	match self {
			CollectionContext::Array		=> write!(f,"Array"),
			CollectionContext::Dict			=> write!(f,"Dict"),
		}
	}
}


/****************************************
****	Foreach Processing Data
*****************************************/
pub struct ForeachData{
	//	iterator counter 
	foreach_iter_counter : Token,

	//	iteration counter detail
	foreach_iter_counter_detail : NormalSymbolEntry,


	//	And the token for the target.  Note we assume the target of a foreach
	//	will always be a "normal" (as opposed to struct member) variable.  Apart
	//	from parsimony, why would you need the target to be a struct member
	foreach_target : Token,
	foreach_target_detail : NormalSymbolEntry,

	//	The token that created the collection detail
	foreach_source : Token,

	//	But the source might be a struct member containing an array so
	//	it's type is "variable".  It might be normal or might be struct member  
	foreach_source_detail : Option<SymbolTableEntryType>,
}

impl ForeachData{
	pub fn new() -> ForeachData{
		ForeachData{
			foreach_iter_counter : Token::new(),
			foreach_iter_counter_detail : NormalSymbolEntry::initialize(),
			foreach_target : Token::new(),
			foreach_target_detail : NormalSymbolEntry::initialize(),
			foreach_source: Token::new(),
			foreach_source_detail : None,	
		}
	}
}

/****************************************
****	Eval Processing Data
*****************************************/

#[derive(PartialEq, Clone, Eq, Hash)]
pub struct EvalData{
	target_var_name : String,
	target_detail : NormalSymbolEntry,
	when_count : usize,
	when_counter : usize,
	otherwise_context : usize,
	otherwise_block_num : usize,
}
impl EvalData{
	pub fn new(when_count : usize, target_var_name : String) -> EvalData{
		EvalData{
			target_var_name : target_var_name,
			target_detail : NormalSymbolEntry::initialize(),
			when_count : when_count,
			when_counter : 0,
			otherwise_context : 0,
			otherwise_block_num : usize::MAX,
		}
	}
}

/****************************************
****	Code Generator
*****************************************/

//	these functions generate the opcodes
pub struct CodeGen<'a>{
	cli : & 'a CLI<'a>,

	pub frames : FrameMap,

	symbol_table : SymbolTable <'a>,
	delimiter_counter : usize,

	//  The context tells us if Array or Dict
	collection_context : Vec<CollectionContext>,

	//	For each loop, while, foreach, contains the address
	//	(frame, address) where a break within those loops will
	//	jump to
	break_address : Vec<(usize,usize)>,
	//	and this is where to go when continue instruction executed
	continue_address : Vec<(usize,usize)>,

	block_begin_counter : usize,

	//	Each element of the holding tank contains the machine instruction for the
	//	function call
	//function_call_hold : Vec<MachineInstruction>,

	//	When an eval is generated this holds all of the context.  It is
	//	needed here because the When AND Otherwise statements will need it
	//	after the eval is initiated.  It's a stack because there might be an Eval
	//	inside an Eval
	eval_data : Vec<EvalData>,

	//	we can't use the symbol table for structs so we construct a mini
	//	symbol table here
	struct_list : Vec<Struct>,
	struct_map : HashMap<String, usize>,
}

impl<'a> CodeGen<'a>{
	pub fn new(cli : & 'a CLI<'a>) -> CodeGen{
		CodeGen{
			cli : cli,
			frames : FrameMap::new(),
			symbol_table : SymbolTable::new(cli),
			delimiter_counter : 0,
			collection_context : Vec::new(),
			block_begin_counter : 0,
			break_address : Vec::new(),
			continue_address : Vec::new(),
			//function_call_hold : Vec::new(),
			eval_data : Vec::new(),
			struct_list : Vec::new(),
			struct_map : HashMap::new(),
		}
	}

	//	Add the declared structs to the local struct map.  So, why do
	//	it this way?
	//
	//	We need a way of connecting an instantiated struct (as in foo = new struct) to
	//	the declared struct and from there to the struct's members.  The instantiated
	//	struct is treated as an array whose elements are its members (hence the requirement
	//	for a linkage to the struct's declaration).  NOTE:  we thought about just installing
	//	each member in the symbol table and treating a member reference just like a normal
	//	variable reference.  This would work except we would need a way to connect a member
	//	back to the instantiated struct so that when we need to copy a struct we could
	//	find all of its members.  We could do this by changing the symbol table into
	//	a BTree and do partial key lookups (i.e. the struct name part of a member name).
	//	But for now, I think I'll stick to ising the struct_map and treat structs as
	//	arrays.
	pub fn add_structs_to_symbol_table(&mut self, structs : &Vec<Struct>){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){
			println!("CodeGen:add_structs_to_symbol_table");
			for s in structs{
				let struct_name = s.name.clone();
				println!("struct {}", struct_name);
				for m in &s.members{
					println!("   {}:{}", struct_name, m.name);
				}
			}
		}

		let mut index = 0;
		for s in structs{
			self.struct_map.insert(s.name.clone(), index);
			self.struct_list.push(s.clone());
			index += 1;
		}
	}

	fn get_random_id(&self) -> String{
		let temp : u16 = random();
		let rtn = format!("$temp{}", temp);
		rtn.clone()
	}

	fn error_abend (&self, error_text : &str){
		println!("CPL abend due to code generation error: {}", error_text);
		std::process::exit(1);
	}


	//	Get the current address of the current block (i.e. the address counter)
	fn get_current_address (&self, function_num : usize) -> usize{
		self.frames.frames_list[function_num].get_current_address()
	}

	fn get_current_block_num(&self, function_num : usize) -> usize{
		self.frames.frames_list[function_num].get_current_block_num()
	}


	fn add_machine_instruction(&mut self, instruction : MachineInstruction, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN_ADD_INSTRUCTION){
			println!("add_machine_instruction: {}:{}", self.frames.frames_list.get_mut(function_num).unwrap().get_address_counter(), instruction);
		}
		self.frames.frames_list.get_mut(function_num).unwrap().add_machine_instruction(instruction);
	}

	fn make_block_current(&mut self, block_num: usize, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("    make_block_current: {} block counter: {}", block_num, self.block_begin_counter);}
		self.frames.frames_list.get_mut(function_num).unwrap().current_code_block_num.push(block_num);

		//	generate the BlockBegin instruction for the new block
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::BlockBegin
				, OpcodeMode::NONE
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, Token::new()
			),function_num
		);
		
		//	add a new symbol table block
		self.symbol_table.push_block();

		//	increment the block_begin_counter (used to do sanity checks)
		self.block_begin_counter += 1;
	}

	//	Add a code block and return its number. DO NOT MAKE THIS BLOCK current block
	//	to add instructions to.  we are only creating the block for later.
	fn add_code_block(&mut self, breakable : bool, function_num : usize) -> usize{
			//	The new block number is the current length of the block_list
		let block_num = self.frames.frames_list[function_num].code_block_list.len();

		//	Add a new block to the block list
		self.frames.frames_list.get_mut(function_num).unwrap().code_block_list.push(CodeBlock::new(breakable));

		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("    add_code_block: added {}", block_num);}

		//	return the block number of the block just created
		block_num
	}

	pub fn code_dump(&self){
		self.frames.code_dump();
	}

	pub fn get_entry_frame_number(&self) -> usize{
		let mut i = 0;
			for f in &self.frames.frames_list{
			if f.is_entry {return i as usize}
			i += 1;
		}

		abend!(format!("from get_entry_frame_number.  Couldn't locate the entry frame"));
	}

	pub fn get_frame_count(&self) -> usize{
		self.frames.frames_list.len()
	}

	pub fn get_frames(&self) -> FrameMap{
		self.frames.clone()
	}

	/****************************************
	***		Function Management Routines  ***
	*****************************************/

	//	look up the function in the frames map and return the frame number and 
	//	builtin flag
	fn get_function_frame_info(&self, function_name : &str) -> (usize,bool){
		if !self.frames.frame_names.contains_key(function_name){
			abend!(format!("From CodeGen.get_function_name: unable to find function {}", function_name));
		}

		*self.frames.frame_names.get(function_name).unwrap()
	}

	//	look up the parameter count from the frame
	fn get_function_parameter_count(&self, function_num : usize) -> usize{
		self.frames.frames_list[function_num].parameters.len()
	}


	//	We add all of the functions to the Frame Map before we generate op codes.  I think this will obviate the
	//	need for forward functions.  We also add SymbolTableFunctions in the symbol table.
	pub fn add_function_to_frame(&mut self, name : &String, function_parameters : &Vec<String>, function_entry_flag : bool){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("CodeGen::ADD_FUNCTION_TO_FRAME function name={}", name);}

		//	Make sure that the function doesn't already exist
		if self.frames.frame_names.contains_key(name){
			abend!(format!("Function {} already defined", name));
		}

		//	Add a new frame to the frame map
		self.frames.add_new_frame(name.to_string(), CodeFrame::new(name, function_entry_flag, false, function_parameters.clone()));
	}

	//	Do a kind of instantiation except we're only adding the information to the
	//	symbol table because the struct has already been built in the operand
	//	stack.  This struct reference is a parameter to a function.  This
	//	this is analagous to parameter_name = new struct_name.
	fn add_struct_parameter_to_symbol_table(&mut self, parameter_name : &String, struct_name : &String){

		//println!("=============== add_struct_parameter_to_symbol_table parameter_name={} struct_name={}",parameter_name,struct_name);
		self.symbol_table.add_normal_symbol(parameter_name);

		//  Get the index of this struct in the struct map
		let struct_index_option = self.struct_map.get(struct_name);
		if struct_index_option == None{
			abend!(format!("from add_struct_to_symbol_table:  Struct {} has not been defined", struct_name));
		}
		let struct_index : usize = *struct_index_option.unwrap();

		//	The index field of this symbol_detail points us to an entry in
		//	the struct_list attribute of codegen.
		let local_struct = self.struct_list[struct_index].clone();
		let local_members = local_struct.members.clone();
		let mut member_index = 0;	

		//	Cycle through the members adding them to the symbol table 
		for m in &local_members{
			//	Add this local reference to the symbol table.
			let member_ref = format!("{}:{}", parameter_name, m.name);

			//println!("====================== add_struct_parameter_to_symbol_table parameter_name={} member_name={}",parameter_name, m.name);
			if self.cli.is_debug_bit(TRACE_CODE_GEN){
				println!("    CodeGen:add_struct_parameter_to_symbol_table:adding {} ({},{}) to symbol table",member_ref, struct_index, member_index);
			}
			self.symbol_table.add_struct_member(member_ref, struct_index, member_index);
			member_index += 1;
		}

		//self.symbol_table.symbol_table_dump_diag("end of add_struct_parameter_to_symbol_table");
	}


	//	Emit instructions to start a function.
	pub fn gen_function(&mut self, name : &String, function_parameters : &Vec<String>, function_entry_flag : bool, function_num : usize, cl_args : &Vec<String>){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("CodeGen::GEN_FUNCTION function name={} num={}", name, function_num);}

		//  add a frame to the symbol table so that block numbers start at 0
		self.symbol_table.add_frame(name);

		//	add code block and the symbol table block for it
		let current_block = self.add_code_block(false, function_num);
		
		//	and make the block current
		self.make_block_current(current_block, function_num);

		//  if this isn't an entry function and there are no parameters, then we're done
		if function_parameters.len() == 0{
			return;
		}

		//  What's happening here is we are adding the parameters to the symbol
		//	table which will use the "current" block and compute the offset within that
		//	block (via the offset of the last instruction added).  The code that
		//	is generated for the function will push the arguments onto
		//	its operand stack block in the same order as they are specified.  Thus, consider:
		//
		//		fn foo(a,b)
		//
		//	The offset of a, will be 0 and b, 1.  Then when its called:
		//
		//		a = foo(10,20)
		//
		//	10 will be pushed onto the operand stack then 20 which, effectively
		//	assigns 10 to a and 20 to b.  These arguments are added to the operand
		//	stack by the function call opcode.
		//
		//	So, where are the arguments?
		//
		//	That depends on if this is an entry function or not.  If this is a
		//	normal function, the function call will have pushed the arguments onto the
		//	the operand stack.
		//
		//	But if this is an entry function, we need to push that data onto the
		//	operand stack just as if "gen_expression" had done it in the function call.
		//
		//	For the entry function there is only ever a single argument which is a CplAarray
		//	containing all of the command line arguments. gen_command_line builds
		//	the CplArray and pushes it onto the operand stack (or, rather, constructs
		//	instructions to do that)
		//
		//  The function call generates opcodes that pushes the arguments onto the
		//  operand stack (in the order in which they were specified).  The address
		//  of these arguments is the current operand block (i.e.
		//	the block that is in effect when the function call is made).  The offsets are
		//	0 through n where n is the number of arguments -1.  The number of arguments
		//	is determined to be (and is confirmed to be true) the number of parameters declared
		//	by the called function.
		if function_entry_flag {
			self.gen_command_line_arg(function_parameters, function_num, cl_args);
			return;
		}
	
		//	add the machine instructions to move the arguments onto the
		//	the operand stack

		//	if this is not the ENTRY function then get the parameters from the operand stack,
		let mut parameter_index : usize = 0;
		while parameter_index < function_parameters.len(){
			//	if the parameter is a qualified id then it MUST be <var>:<struct name>.  In
			//	particular, it is NOT a struct member reference.  Instead it is a "type" (the
			//	only place in this mess where the CPL programme specifies a type).  It is
			//	roughly equivalent to <var> = new <struct name> except we aren't creating
			//	the instanstiated struct in memory, we are only making sure that the
			//	symbol table is constructed so references can be found.

			let parts: Vec<&str> = function_parameters[parameter_index].split(':').collect();

			//	If this is a qualified ID then we need to add the struct information
			//	to the symbol table
			if parts.len() > 1{
				self.add_struct_parameter_to_symbol_table(&parts[0].to_string(), &parts[1].to_string());
				let symbol_detail = self.symbol_table.add_normal_symbol(&parts[0].to_string());
				//	Push the argument onto the operand stack
				self.add_machine_instruction(
					MachineInstruction::new(
						Opcode::Push
						, OpcodeMode::Arg
						, self.symbol_table.current_frame()
						, symbol_detail.block_num
						, symbol_detail.index
						, Vec::new()
						, Token::new()
					),function_num
				);
			}else{
				//	And we still need to deal with the parameter name itself
				let symbol_detail = self.symbol_table.add_normal_symbol(&parts[0].to_string());

				//	Push the argument onto the operand stack
				self.add_machine_instruction(
					MachineInstruction::new(
						Opcode::Push
						, OpcodeMode::Arg
						, self.symbol_table.current_frame()
						, symbol_detail.block_num
						, symbol_detail.index
						, Vec::new()
						, Token::new()
					),function_num
				);
			}


			parameter_index += 1;
		}
	}

	/****************************************
	***		Command Line Interface        ***
	*****************************************/

	//	In order to get arguments from the command line onto the operand stack...
	fn gen_command_line_arg(&mut self, function_parameters : &Vec<String>, function_num : usize, cl_args : &Vec<String>){
		//	Add the parameter to the symbol table.  For the entry function there is only 1 parameter
		//	It is "normal" because it's not a struct (ha ha);
		self.symbol_table.add_normal_symbol(&function_parameters[0]);

		//	Creates a new empty array in the operand stack.  The address is created in
		//	in the symbol table by the previous call to "add_normal_symbol")
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::PushNewCollection
				, OpcodeMode::Array		// constructs a CplArray with no elements
				, self.symbol_table.current_frame()
				, 0						// the command line args always at block 0
				, 0						// command line arg array at address 0
				, Vec::new()
				, Token::new()
			),function_num
		);

		//	And now populate it with values from the command line arguments vector.  Each of
		//	these values is a string from the space separate list.
		for arg in cl_args{
			//	Need the argument as a token (by the time we see it it's still a string)
			let token = Token::new2(
				TokenType::STRING
				, arg.clone()
				, 0		// line number 0 (i.e. prior to any code)
				, "Command Line Argument".to_string()
				, TokenCategory::Factor
			);

			//	push the newly minted argument token onto the stack.  Each time we do this
			//	it'll end up just after the array itself
			self.add_machine_instruction(
				MachineInstruction::new(
					Opcode::Push
					, OpcodeMode::Lit
					, self.symbol_table.current_frame()
					, 0
					, 0
					, Vec::new()
					, token.clone()
				),function_num
			);	

			//	And, now we can update the array.  This is a direct update (as
			//	opposed to via a VarRef)
			self.add_machine_instruction(
				MachineInstruction::new(
					Opcode::Update
					, OpcodeMode::Array		//push tos onto array at tos
					, self.symbol_table.current_frame()
					, 0
					, 0
					, Vec::new()
					, Token::new()
				),function_num
			);	
		}
	}




	/****************************************************************************
	***		Generate instructions that implement staements and expressions    ***
	*****************************************************************************/
	//	The block end instruction is the light-weight return to the block that
	//	called it.  It also pops the operand stack block.
	pub fn gen_block_end (&mut self, function_num : usize){
		//	if the block_counter is already at zero and the current_code_block_num stack
		//	is empty then this is end of function stuff otherwise it's an error
		if self.block_begin_counter > 0{
			self.block_begin_counter -= 1;
		}else if !self.frames.frames_list[function_num].current_code_block_num.is_empty(){
			abend!(format!("Error ********* from gen_block_end:  block_begin_counter = 0 and current_code_block_num stack is not empty"));
		}

		if self.cli.is_debug_bit(TRACE_CODE_GEN){
			println!("CodeGen::GEN_BLOCK_END count {}",self.block_begin_counter);
		}

		//  If we're all done with the function, clear the break and continue
		//	addresses
		if self.block_begin_counter == 1{
			self.break_address.clear();
			self.continue_address.clear();
		}

		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::BlockEnd
				, OpcodeMode::NONE
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, Token::new()
			),function_num
		);

		self.symbol_table.symbol_table_dump();
		self.symbol_table.pop_block();

		//	and go back to adding instructions to the original
		//	code block
		self.frames.frames_list.get_mut(function_num).unwrap().current_code_block_num.pop();
	}

	//	A factor can be an ID, QUAIFIED_ID (i.e. a struct member reference), a number or a boolean
	//	litearl (i.e. true or false
	fn gen_expression_factor(&mut self, token : &Token, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("    gen_expression_factor: {}", token);}

		match token.token_type{
			TokenType::ID | TokenType::INDEXED_ID | TokenType::IDADDR | TokenType::QUALIFIED_ID => self.gen_expression_id(token, function_num),
			TokenType::INTEGER | TokenType::FLOAT | TokenType::STRING  | TokenType::BOOL => self.gen_expression_scalar(token, function_num),
			_ => abend!(format!("from gen_expression_factor:  Houston we have a problem.  Factors not mapped correctly.{}", token.token_type)),			
		}
	}

	//	Struct references are normally, <instantiated struct>:<struct member>.  This
	//	translated into an array reference (instantiated structs are arrays whose
	//	elements are the members of the struce).  However, if all we see is
	//	<instantiated struct>: then we are assuming the pcl program is referencing the
	//	the entire struct (e.g. in a function call or just copying it).
	fn gen_expression_struct_member(&mut self, token : &Token, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("    gen_expression_struct_member: {}", token);}

		//self.symbol_table.symbol_table_dump_diag("at gen_expression_struct_member");


		//	first we need to split this into the struct part and the member part
		let parts: Vec<&str> = token.token_value.split(':').collect();
		let struct_name = parts[0].to_string();

		//	get the address of the instantiated struct (which is really an array)
		//	The payload in the symbol table is a normal entry
		//println!("===============gen_expression_struct_member struct_name={}", struct_name);
		let struct_detail = self.symbol_table.get_normal_address(&struct_name);

		//	push the instantiated struct onto the operand stack (this
		//	is the array that implements the instantiated struct)
		self.gen_expression_struct(token, struct_detail.block_num, struct_detail.index, function_num);

		//	if this is a struct reference (i.e. <name>: without any member), as oppose to a struct member reference,
		//	I think we just push push the struct (i.e. the array) onto the operand stack as opposed to
		//	pushing a indexed array access onto the operand stack.
		if parts[1].len() == 0{
			let mut ref_token = token.clone();
			ref_token.token_type = TokenType::STRUCT;
			self.gen_expression_struct(&ref_token, struct_detail.block_num, struct_detail.index, function_num);
			return;
		}
		
		//	When the struct was instantiated, the entry that was added contained
		//	the struct index (in the list of structs in the model) and the index of
		//	the particular member in the array (i.e. the ordinal number of the member);
		let member_entry = self.symbol_table.get_struct_member_entry(&token.token_value);

		//	Construct a token containing the index
		let mut index_token = token.clone();
		index_token.token_value = member_entry.member_index.to_string();
		index_token.token_category = TokenCategory::Factor;
		index_token.token_type = TokenType::INTEGER;
		
		//	Now push the index onto the stack
		self.gen_expression_scalar(&index_token, function_num);

		//  Emit the fetch indexed instruction
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::FetchIndexed
				, OpcodeMode::NONE
				, self.symbol_table.current_frame()
				, 0			
				, 0
				, Vec::new()
				, token.clone()
			),function_num
		);
	}


	//	If the token is either an ID or IDADDR (e.g. &x), grab it and push it onto the
	//	operand stack.  For example a = b will generate this as will foo(&x).  The mode
	//	for the former is "Var" and "VarRef" for the latter.
	fn gen_expression_id_detail(&mut self, token : &Token, symbol_detail : &NormalSymbolEntry, function_num : usize){
		let mode : OpcodeMode;
		if token.token_type == TokenType::IDADDR{
			mode = OpcodeMode::VarRef;
		}else{
			mode = OpcodeMode::Var;
		}

		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Push
				, mode
				, self.symbol_table.current_frame()
				, symbol_detail.block_num			// block num
				, symbol_detail.index 				// address in block
				, Vec::new()
				, token.clone()
			),function_num
		);
	}

	//	If the ID was the name of an instantiated struct, we want to push a VarRef
	//	pointing to it onto the stack.
	fn gen_expression_struct(&mut self, token : &Token, symbol_block_num : usize, symbol_index : usize, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("    gen_expression_struct: {}", token);}

		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Push
				, OpcodeMode::VarRef
				, self.symbol_table.current_frame()
				, symbol_block_num			// block num
				, symbol_index 				// address in block
				, Vec::new()
				, token.clone()
			),function_num
		);
	}

	fn gen_expression_id(&mut self, token : &Token, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("    gen_expression_id: {}", token);}

		//	let's look at the id (token) in the symbol table because it might be either
		//	a normal symbol or a struct

		//self.symbol_table.print_entry_type(&token.token_value);

		let entry = self.symbol_table.get_symbol_entry(&token.token_value);
		// match entry{
		// 	None => {}
		// 	Some(ref e) => {}
		// }
	
		match entry {
			None => abend!(&format!("From gen_expression_id: {} Not in symbol table",token.token_value)),
			Some(ref e) => match e{
				SymbolTableEntryType::NormalSymbolEntry(normal_symbol_detail) => {
					// println!("==================== gen_expression_id entry={}",e);
					self.gen_expression_id_detail(token, &normal_symbol_detail, function_num);
				},

				SymbolTableEntryType::StructMemberEntry(_) => {
					//println!("==================== gen_expression_id entry={}",e);
					self.gen_expression_struct_member(&token, function_num);
				},

				SymbolTableEntryType::StructEntry(struct_detail) => {
					// println!("==================== gen_expression_id entry={}",e);
					self.gen_expression_struct(token, struct_detail.block_num, struct_detail.index, function_num);
				}
			}

		}
	}

	fn gen_expression_scalar(&mut self, token : &Token, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("    gen_expression_scalar: {}", token);}

		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Push
				, OpcodeMode::Lit
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, token.clone()
			),function_num
		);		
	}

	// fn gen_expression_bool (&mut self, token : &Token, function_num : usize){
	// 	self.add_machine_instruction(
	// 		MachineInstruction::new(
	// 			Opcode::Push
	// 			, OpcodeMode::Lit
	// 			, self.symbol_table.current_frame()
	// 			, 0
	// 			, 0
	// 			, Vec::new()
	// 			, token.clone()
	// 		),function_num
	// 	);		
	// }


	// fn gen_xxx_function_call_release(&mut self){
	// 	let mi = self.function_call_hold.pop().unwrap();
	// 	self.add_machine_instruction(mi);
	// }

	fn gen_expression_function_call(&mut self, token : &Token, function_num : usize){
		//  For function calls in an expression, we build machine instructions
		//	in the order they appear in the postfix express.
		
		//  So, we build the function call
		//	instruction, and push it onto a holding tank.  At the top of the loop, the tank is
		//	queried along with an item counter.  When the item counter has gone to zero, the
		//	function call instruction is added to the opcode list.

		//	get the info from the function frame (constructed by the model)
		let target_frame_info = self.get_function_frame_info(&token.token_value);
		//	get the number of parameters defined by the called function
		let target_parameter_count = self.get_function_parameter_count(target_frame_info.0);
		//	extract the number of arguments defined by the function call and if it
		//	doesn't match the number of parameters of the called routine, die with an error
		if let TokenType::FUNCTION_CALL(arg_count) = token.token_type{
			if arg_count != target_parameter_count{
				abend!(format!("From gen_expression_function_call:  Argument count ({}) doesn't match parameter count ({}) for function {}", arg_count, target_parameter_count, token.token_value));
			}
		}

		//	define whether or not the function is internal or builtin
		let opcode_mode : OpcodeMode;
		if target_frame_info.1 {
			opcode_mode = OpcodeMode::Extern;
		}else{
			opcode_mode = OpcodeMode::Function;
		}

		//	Construct a machine instruction
		let fc = MachineInstruction::new(
			Opcode::FunctionCall
			, opcode_mode
			, self.symbol_table.current_frame()
			, target_frame_info.0				// frame number of called function
			, function_num							// for function calls this is the frame where the instruction needs to go
			, vec!(target_parameter_count, 0)	// number of parameters declared, is_statement = false
			, token.clone()						// function name
		);

		self.add_machine_instruction(fc, function_num);
	}

	//	When the token in the expression is a "list separator" type comma, we
	//	are constructing an array via a literal array (e.g. [1,2,3]) and this
	//	causes the update of the array.  The array is at the top of the stack
	//	And will never be "popped".
	fn gen_expression_comma(&mut self, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("    gen_expression_comma");}
		if self.collection_context.len() > 0{
			//	only update if context is array.  Dictionary updates are triggered by RDict tokens
			if let CollectionContext::Array = self.collection_context.last().unwrap(){
				//	update the array variable at the top of the stack
				self.add_machine_instruction(
					MachineInstruction::new(
						Opcode::Update
						, OpcodeMode::Array
						, self.symbol_table.current_frame()
						, 0
						, 0
						, Vec::new()
						, Token::new()
					),function_num
				);							
			}
		}
	}

	fn gen_expression_lbracket(&mut self, function_num : usize){
		//	if this is the first time we've seen this, set the context to
		//	array.
		if self.delimiter_counter == 0{
			self.collection_context.push(CollectionContext::Array);
		}
		self.delimiter_counter += 1;

		//	Create an array at the current TOS
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::PushNewCollection
				, OpcodeMode::Array
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, Token::new()
			),function_num
		);							
	}


	//	Evidently we only see a "]" at the end of a literal array expression  If
	//	we see a Rindex ("]") then it'll be at the end of an index expression
	//	We are creating a new array to be populated via a literal expression.
	//	it'll never be popped.
	fn gen_expression_rbracket(&mut self, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("    gen_expression_rbracket: delimiters={}", self.delimiter_counter);}

		self.delimiter_counter -= 1;

		//	if the delimiter counter goes to zero, then we're done with this
		//	context
		if self.delimiter_counter == 0{
			self.collection_context.pop();
		}

		//	at the end of the array, update whatever is at TOS-1 with
		//	whatever is at TOS.  This is a direct update.
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Update
				, OpcodeMode::Array
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, Token::new()
			),function_num
		);	
	}

	fn gen_expression_rindex(&mut self, token : &Token, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("    gen_expression_rindex: {}", token);}
		//  At this point TOS is an index and TOS-1 is an array.  The rindex (i.e. "]"
		//	triggers the fetch_indexed exec function).
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::FetchIndexed
				, OpcodeMode::NONE
				, self.symbol_table.current_frame()
				, 0			
				, 0
				, Vec::new()
				, token.clone()
			),function_num
		);
	}

	//	This is the bginning of a dictionary literal.  Create a new empty dictionary
	//	at the top of the stack
	fn gen_expression_ldict(&mut self, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("    gen_expression_ldict");}
		self.collection_context.push(CollectionContext::Dict);

		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::PushNewCollection
				, OpcodeMode::Dict
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, Token::new()
			),function_num
		);
	}

	//	This is the end of a label_value pair, add the key and value to
	//	the dictionary.  This is triggered via the construction of dictionary
	//	via a literal dictionary expression
	fn gen_expression_rdict_kv(&mut self, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("    gen_expression_rdict_kv");}
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Update
				, OpcodeMode::Dict
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, Token::new()
			),function_num
		);

	}
	
	//	This is the end of a dictionary literal
	fn gen_expression_rdict(&mut self, _frame_num : usize){
		self.collection_context.pop();
	}

	fn gen_expression(&mut self, expression_list : &Vec<Token>, function_num : usize){

		for t in expression_list{
			if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("    gen_expression: driver {}", t);}

			match t.token_category{
				TokenCategory::Factor | TokenCategory::IndexedId => self.gen_expression_factor(&t, function_num),
				TokenCategory::FunctionCall => self.gen_expression_function_call(&t, function_num),
				TokenCategory::ListSeparator => self.gen_expression_comma(function_num),
				TokenCategory::LBracket => self.gen_expression_lbracket(function_num),
				TokenCategory::RBracket => self.gen_expression_rbracket(function_num),
				TokenCategory::RIndex => self.gen_expression_rindex(&t, function_num),
				TokenCategory::LDict => self.gen_expression_ldict(function_num),
				TokenCategory::RDict => self.gen_expression_rdict(function_num),
				TokenCategory::RDict_Kv => self.gen_expression_rdict_kv(function_num),

				//	These are zombie tokens that the infix to postfix conversion adds to
				//	the postfix expression but which we don't have to do anything about.
				TokenCategory::LIndex => {},
				TokenCategory::LDict_Kv => {},
				TokenCategory::LParen_Arg => {},
				TokenCategory::RParen_Arg => {},

				_ =>{
					self.add_machine_instruction(MachineInstruction::new(
						token_type_to_opcode(t.token_type)
						, OpcodeMode::NONE
						, self.symbol_table.current_frame()
						, 0
						, 0
						, Vec::new()
						, t.clone()),function_num
					);
				},
			}
		}
	}

	pub fn gen_loop(&mut self, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("gen_loop");}

		//	This is the block where the return instruction goes to
		let return_block_num = self.get_current_block_num(function_num);

		//	this is the target block for the while block
		let while_block_num = self.add_code_block(true, function_num);

		//	Mark the current address
		let pre_condition_address = self.get_current_address(function_num);

		self.break_address.push ((return_block_num, pre_condition_address + 2));
		self.continue_address.push ((return_block_num, pre_condition_address + 1));

		//	post_condition_address + 1 call the while loop, return to the next location
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Bl
				, OpcodeMode::Bl
				, self.symbol_table.current_frame()
				, return_block_num
				, pre_condition_address + 1		// at this offset
				, vec!(while_block_num)			// bl target
				, Token::new()
			),function_num
		);
		
		//	post_condition_address + 2:   jump to pre_condition_address
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::J
				, OpcodeMode::Jump
				, self.symbol_table.current_frame()
				, 0
				, pre_condition_address
				, Vec::new()
				, Token::new()
			),function_num
		);

		self.make_block_current(while_block_num, function_num);
	}
	
	//	The operand of a simple opcode is an expression:  compute it, operate on it, and pop it when
	//	its done.

	//	BUT! if the expression_list is empty then the instruction is probably
	//	break or continue (which are both "simple")

	//	*************************************************************************************
	//	TODO!!!   if the expression contains variables that are actually parameters, then
	//	we don't need to push them (or pop them afterwords).
	//	*************************************************************************************
	
	pub fn gen_simple(&mut self, token : &Token, expression_list : &Vec<Token>, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("CodeGen:GEN_SIMPLE: {} {}",token.token_value, token_list_text(expression_list));}
		
		//	If the expression list is empty then the statement better be either BREAK or CONTINUE
		if expression_list.is_empty() 
			&& token.token_type != TokenType::BREAK
			&& token.token_type != TokenType::CONTINUE {
			abend!(format!("From gen_simple: expected BREAK or CONTINUE but got: {}", token.token_type));
		} else {
			self.gen_expression(expression_list, function_num);
		}


		//	if we're breaking or continuing, make sure that we're inside some kind of
		//	breakable block.  Start at the current (last) block and move up the chain
		//	until either we found a breakable block or we hit the end.
		if token.token_type == TokenType::BREAK || 	token.token_type == TokenType::CONTINUE{
			let mut block = (self.frames.frames_list[function_num].code_block_list.len()) as i32 - 1;

			loop{
				if block < 0{
					self.error_abend(
						&format!("FROM gen_simple. {} Not allowed in this context: PCL Line {}", token.token_type, token.line_number)
					);		
				}
				if self.frames.frames_list[function_num].code_block_list[block as usize].breakable{
					break;
				}
				block -= 1;
			}
		}

		match token.token_type {
			TokenType::BREAK => {
				let break_address = self.break_address.last().unwrap();

				self.add_machine_instruction(
					MachineInstruction::new(
						  Opcode::Break
						, OpcodeMode::NONE
						, self.symbol_table.current_frame()
						, break_address.0
						, break_address.1
						, Vec::new()
						, token.clone()
					),function_num
				);	
			},

			TokenType::CONTINUE => {
				let continue_address = self.continue_address.last().unwrap();
				self.add_machine_instruction(
					MachineInstruction::new(
						  Opcode::Continue
						, OpcodeMode::NONE
						, self.symbol_table.current_frame()
						, continue_address.0
						, continue_address.1
						, Vec::new()
						, token.clone()
					),function_num
				);	
			}

			_=> self.add_machine_instruction(
				MachineInstruction::new(
					  token_type_to_opcode(token.token_type)
					, OpcodeMode::NONE
					, self.symbol_table.current_frame()
					, 0
					, 0
					, Vec::new()
					, token.clone()
				),function_num
			),
		}
	}


	/*
		An assignment statement either creates a new variable on the stack or updates
		an existing one using one of the various assignment operators.  If the target
		exists, then:

			<assignment operator>

		replaces what's there.

		So, how do we know whether to add a new value or update an existing one?

		1.	check the symbol table and if the variable doesn't exist then
			create it
		2.	Otherwise, update it.

		How does gen_expression know to create or update.  It doesn't know.  It just
		blindly leaves the result of the  expression on the stack.  It is up to
		this code to either leave it there or use it to update the existing one.

		if the length of the target_index_expression is > 0 then it means that
		we need emit code to update an element of array, rather than a scalar
	*/

	fn op_to_opcode (&self, op : &Token) -> Opcode{
		match op.token_type{
			TokenType::ASG_ADD_EQ  		=> Opcode::AddEq,
			TokenType::ASG_SUB_EQ  		=> Opcode::SubEq,
			TokenType::ASG_DIV_EQ  		=> Opcode::DivEq,
			TokenType::ASG_MUL_EQ  		=> Opcode::MulEq,
			TokenType::ASG_MOD_EQ 		=> Opcode::ModEq,
			TokenType::ASG_APPEND_EQ  	=> Opcode::AppendEq,
			TokenType::ASG_OR_EQ 		=> Opcode::OrEq,
			TokenType::ASG_AND_EQ  		=> Opcode::AndEq,
			TokenType::ASG_EQ 			=> Opcode::Update,

			_=> abend!(format!("From op_to_opcde:  {} is an unknown assignment operator", op.token_type)),
		}		
	}
	fn gen_struct_instantiate_push_uninitialized(&mut self, token : &Token, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){
			println!("    gen_struct_instantiate_push_uninitialized target={}", token.token_value);
		}
		//	Add a CplUninitialized to the array
		let mut uninit = token.clone();
		uninit.token_type = TokenType::NONE;
		uninit.token_category = TokenCategory::Unknown;
		uninit.token_value = "UNINITIALIZED".to_string();

		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Push
				, OpcodeMode::Lit
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, uninit.clone()
			),function_num
		);
	}


	//	emit the code to instantiate a struct:
	//
	//		instantiated_struct is the name of the target variable
	//		struct_name is the declared struct (i.e. template)
	//
	pub fn gen_struct_instantiate(&mut self, instantiated_struct : &Token, struct_name : &Token, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){
			println!("CodeGen:GEN_STRUCT_INSTANTIATE target={} struct={}", instantiated_struct.token_value, struct_name.token_value);
		}

		//	So what happens here is that the target becomes an array.  The
		//	number of elements in that array is the number of members of the struct
		//	We'll add the following to the symbol table (at whatever block we're in
		//	now) for each member of the struct.
		//
		//		<target>:<member>
		//
		//  In the symbol table, an instatiated struct appears as a struct member without
		//	any member (e.g. foo:).  This is so that when we look for an instsantiated
		//	struct in the symbol table we can distinguish it from a normal local variable
		let instantiated_struct_symbol = format!("{}",instantiated_struct.token_value);
		let struct_detail = self.symbol_table.add_normal_symbol(&instantiated_struct_symbol);

		//	and allocate an undefined variable
		self.gen_alloc(&instantiated_struct, struct_detail.block_num, struct_detail.index, function_num);

		//	push an empty array onto the operand stack.  This array will end up containing
		//	all of the members
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::PushNewCollection
				, OpcodeMode::Array
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, Token::new()
			),function_num
		);

		
		//	Now, for each member, push the member onto the newly created
		//	array.  Recall that an instantiated struct is just an array
		//	whose elements are the members defined in the struct declaration and
		//	whose value is defined by the initializer.  If no initializer is
		//	present then the value is CplUndefined.
		

		//	get the index of the struct in the struct 
		let struct_index_option = self.struct_map.get(&struct_name.token_value);
		if struct_index_option == None{
			panic!("from gen_struct_instantiate:  Struct {} has not been defined", struct_name.token_value);
		}
		let struct_index : usize = *struct_index_option.unwrap();

		//	The index field of this symbol_detail points us to an entry in
		//	the struct_list attribute of codegen.
		let local_struct = self.struct_list[struct_index].clone();
		let local_members = local_struct.members.clone();
		let mut member_index = 0;	

		//	Cycle through the members adding them to the symbol table and updating
		//	the array at the top of the operand stack.
		for m in &local_members{
			//	If the member of the struct has an initializer, emit those instructions as well.
			if m.initializer.len() == 0{
				self.gen_struct_instantiate_push_uninitialized(instantiated_struct, function_num);
			}else{
				//println!("=================== gen member {:?}",m.initializer);
				self.gen_expression(&m.initializer, function_num)
			}

			//	Add this local reference to the symbol table.  Note that details for
			//	a struct member contains the index of the member in the instatiated
			//	struct.  And, since there isn't an "address" apart from the index
			//	the entry does not take up any space in the operand stack
			let member_ref = format!("{}:{}", instantiated_struct.token_value, m.name);
			if self.cli.is_debug_bit(TRACE_CODE_GEN){
				println!("    CodeGen:GEN_STRUCT_INSTANTIATE:adding {} ({},{}) to symbol table",member_ref, struct_index, member_index);
			}
			self.symbol_table.add_struct_member(member_ref, struct_index, member_index);

			//	Push the member onto the array at tos
			self.add_machine_instruction(
				MachineInstruction::new(
					Opcode::Update
					, OpcodeMode::Array
					, self.symbol_table.current_frame()
					, 0
					, 0
					, Vec::new()
					, Token::new()
				),function_num
			);

			member_index += 1;
		}			

		//	Turn the allocated variable into the array; update from
		//	the operand stack
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Update
				, OpcodeMode::Update
				, self.symbol_table.current_frame()
				, struct_detail.block_num
				, struct_detail.index
				, Vec::new()
				, instantiated_struct.clone()
			),function_num
		);
	}

	//	Assignment to a struct member has to look like assignment to an
	//	an element of the array.  We'll need:  the address of the instantiated struct,
	//	an index token that we have to create and an expression vector whose first element
	//	is the index token.  the update is in situ (i.e. at a specified location in the stack)
	
	//	TODO:  This code is remarkably similar to the gen_assignment_to_array that we should be able
	//	reduce the duplication.
	fn gen_assignment_to_struct_member(&mut self, target : &Token, op : &Token, expression_list : &Vec<Token>, function_num : usize){
		let parts: Vec<&str> = target.token_value.split(':').collect();
		let struct_name = parts[0].to_string();
		//struct_name.push(':');

		//	get the address of the instantiated struct which is really an array
		let detail = self.symbol_table.get_normal_address(&struct_name);

		//	get the index of the member from the symbol table
		let member_entry = self.symbol_table.get_struct_member_entry(&target.token_value);
		//	and build a token that can be used in an expression that
		//	creates the index
		let mut index_token = target.clone();
		index_token.token_type = TokenType::INTEGER;
		index_token.token_value = member_entry.member_index.to_string();

		//	Add the instruction push the index on to the operand stack
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Push
				, OpcodeMode::Lit
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, index_token.clone()
			),function_num
		);
			
		//	set the local id type to INDEXED
		let mut local_target = target.clone();
		local_target.token_type = TokenType::INDEXED_ID;
		local_target.token_category = TokenCategory::IndexedId;
			
		//	The new value is at the top of the operand stack
		self.gen_expression(expression_list, function_num);

		//	And we want to update the member element of the struct
		self.add_machine_instruction(
			MachineInstruction::new(
				self.op_to_opcode(op)
				, OpcodeMode::UpdateIndexed
				, self.symbol_table.current_frame()
				, detail.block_num
				, detail.index
				, Vec::new()
				, target.clone()
			),function_num
		);
	
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("CodeGen:GEN_ASSIGNMENT_TO_STRUCT_MEMBER target={}", target.token_value);}
	}

	fn gen_assignment_to_scalar(&mut self, target : &Token, op : &Token, expression_list : &Vec<Token>, function_num : usize){
		let detail : NormalSymbolEntry;
		
		match self.symbol_table.get_symbol_entry(&target.token_value){
			None		=> {
				//	add the symbol and return its detail
				detail = self.symbol_table.add_normal_symbol(&target.token_value);

				//	Generate the alloc for the new symbol
				self.gen_alloc(&target, detail.block_num, detail.index, function_num);
			},
			//	the symbol exists so just get its detail	
			Some (entry) 	=> {
				match entry{
					SymbolTableEntryType::NormalSymbolEntry(d) => detail = d,

					//	If the variable was used previously as a struct instantiation then we 
					//	can use its slot in the operand stack but need to change the detail
					//	type from Struct to normal and update the detail values
					SymbolTableEntryType::StructEntry(m) => {
						detail = NormalSymbolEntry::new(m.block_num, m.index);
					}
					_ => abend!(format!("From gen_assignment_to_scalar: Symbol {} Exists but is not the correct type", target.token_value)),
				}
			},
		}

		//	The new value is at the top of the operand stack
		self.gen_expression(expression_list, function_num);

		self.add_machine_instruction(
			MachineInstruction::new(
				self.op_to_opcode(op)
				, OpcodeMode::Update
				, self.symbol_table.current_frame()
				, detail.block_num
				, detail.index
				, Vec::new()
				, target.clone()
			),function_num
		);

		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("CodeGen:GEN_ASSIGNMENT_TO_SCALAR target={}", target.token_value);}
	}

	//	Since this is the "lvalue" we need to update in situ.
	fn gen_assignment_to_collection (&mut self, target : &Token, op : &Token, target_index_expression : &Vec<Token>,expression_list : &Vec<Token>, function_num : usize){
		let detail : NormalSymbolEntry;
		let mut local_id = target.clone();

		detail = self.symbol_table.get_normal_symbol_entry(&target.token_value);

		//	set the local id type to INDEXED
		local_id.token_type = TokenType::INDEXED_ID;
		local_id.token_category = TokenCategory::IndexedId;
		
		//	emit the code that will compute the index
		//	Generate the instructions to process the expresion
		self.gen_expression(target_index_expression, function_num);

		//	The new value is at the top of the operand stack
		self.gen_expression(expression_list, function_num);

		self.add_machine_instruction(
			MachineInstruction::new(
				self.op_to_opcode(op)
				, OpcodeMode::UpdateIndexed
				, self.symbol_table.current_frame()
				, detail.block_num
				, detail.index
				, Vec::new()
				, target.clone()
			),function_num
		);

		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("CodeGen:GEN_ASSIGNMENT_TO_ARRAY target={}", target.token_value);}
	}

	pub fn gen_assignment(&mut self, target : &Token, op : &Token, target_index_expression : &Vec<Token>,expression_list : &Vec<Token>, function_num : usize){		
		//	And, if this is an assignment to a struct member got more hacking
		//	to do.
		if target.token_type == TokenType::QUALIFIED_ID{
			self.gen_assignment_to_struct_member(target, op, expression_list, function_num);
			return;
		}

		//	target is not a struct member.  Is it a indexed expression (i.e. target is an array
		//	or dictionary element)?
		if target_index_expression.len() == 0{
			self.gen_assignment_to_scalar(target, op, expression_list, function_num);
		}else{
			self.gen_assignment_to_collection(target, op, target_index_expression, expression_list, function_num);
		}
	}

	pub fn gen_function_call_statement(&mut self, function_name : &Token, expression_list : &Vec<Token>, function_num : usize, argument_count : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("CodeGen::GEN_FUNCTION_CALL_STATEMENT:  target={}",function_name.token_value);}

		//	find the function name in the frame map
		if self.frames.frame_names.contains_key(&function_name.token_value){
			if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("CodeGen:gen_function_call_statement: {} block {}",function_name.token_value, self.frames.frame_names.get(&function_name.token_value).unwrap().0);}
		}else{
			abend!(format!("From gen_function_call_statement: function name {} not found", function_name.token_value));
		}

		//	Generate the arguments expression, which, when executed will
		//	place the number variables on the operand stack as there were
		//	arguments.  The function call instruction will move these to the
		//	argument list of the called function
		self.gen_expression(expression_list, function_num);

		//	Generate the actual call

		let target_frame_info = self.get_function_frame_info(&function_name.token_value);
		let target_parameter_count = self.get_function_parameter_count(target_frame_info.0);

		if argument_count != target_parameter_count{
			abend!(format!("From gen_function_call_statement: Argument count ({}) does't match parameter count ({}) for function {}", argument_count, target_parameter_count, function_name.token_value));
		}

		let opcode_mode : OpcodeMode;
		if target_frame_info.1 {
			opcode_mode = OpcodeMode::Extern;
		}else{
			opcode_mode = OpcodeMode::Function;
		}
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::FunctionCall
				, opcode_mode
				, self.symbol_table.current_frame()
				, target_frame_info.0			// frame number of called function
				, 0
				, vec!(target_parameter_count, 1) // number of parameters declared and is_statement=true
				, function_name.clone()			// function name
			),function_num
		);

		//	Since this is a statement (and not a term of an expression), we
		//	remove the return from called function (NOTE the called function doesn't
		//	know if it was called from a statement or from a factor in an expression)
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Pop
				, OpcodeMode::NONE
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, Token::new()
			),function_num
		);
	}

	//	exec alloc instruction examines the specified location in the operand stack and, if there is
	//	already something there, does nothing.  If the location in the operand stack hasn't been
	//	used it creates and uninitialized variable at that location.
	fn gen_alloc(&mut self, target : &Token, detail_block_num : usize, detail_index : usize, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){
			println!("    gen_alloc: {} : {}:{}", target,detail_block_num,detail_index);
		};

		//	creates an uninitialized variable at the address frame,block,address
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Alloc
				, OpcodeMode::Alloc			//  this is equivalent to NONE but allows for better diag info
				, self.symbol_table.current_frame()
				, detail_block_num
				, detail_index
				, Vec::new()
				, target.clone()
			),function_num
		);		
	}

	/************************************************************************
	***		Generate instructions that implement "blockable" statements   ***
	*************************************************************************/

	pub fn gen_eval(&mut self, target : &Vec<Token>, when_count : usize, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("CodeGen:GEN_EVAL: count={} target={} block_count={}", when_count, token_list_text(&target), self.block_begin_counter);}
		if self.cli.is_debug_bit(INSERT_DIAG_COMMENTS){
			self.add_machine_instruction(
				MachineInstruction::new(
					Opcode::Diag
					, OpcodeMode::NONE
					, self.symbol_table.current_frame()
					, 0
					, 0
					, Vec::new()
					, Token::make_string(&format!("=== gen_eval === {}",target[0].line_text))
				),function_num
			);
		}

		let current_block_num = self.get_current_block_num(function_num);

		//	create a local instance of EvalData using a random name for the target
		let mut eval_data = EvalData::new(when_count,self.get_random_id());

		//	add a block to the block list.  save its number.  We probably can
		//	change this to current_block + 1 -- let's wait and see...
		let eval_block_num = self.add_code_block(false, function_num);

		//	create a target variable
		//let target_var_name = self.get_random_id();
		//eval_data.target_var_name = self.get_random_id();

		//	the target token to add to the operand stack
		let target_var = Token::new2(
			TokenType::ID
			, eval_data.target_var_name.clone()
			, 0
			, "ha ha".to_string()
			, TokenCategory::Factor
		);

		//	Create a target variable; fail if it already exists
		match self.symbol_table.get_symbol_entry(&eval_data.target_var_name){
			None		=> {
				//	add the symbol and return its detail
				eval_data.target_detail = self.symbol_table.add_normal_symbol(&eval_data.target_var_name);

				//	Generate the alloc for the new symbol
				self.gen_alloc(&target_var, eval_data.target_detail.block_num, eval_data.target_detail.index, function_num);
			},
			//	the symbol should not already exist	(because it's a random name!)
			Some (_) 	=> abend!(format!("from gen_eval: Houston, we have a problem!  The random name {} is already in the symbol table", eval_data.target_var_name)),
		}

		//	generate the condition evaluation code
		self.gen_expression(target, function_num);

		//	update the target variable with whatever expression computed
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Update
				, OpcodeMode::Update
				, self.symbol_table.current_frame()
				, eval_data.target_detail.block_num
				, eval_data.target_detail.index
				, Vec::new()
				, target_var
			),function_num
		);

		//	add eval data to the stack
		self.eval_data.push(eval_data.clone());

		//	get the current instruction address
		let current_address = self.get_current_address(function_num);

		//	branch and link to the eval block
		//	current address + 1:  call the if block return to the next instruction
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Bl
				, OpcodeMode::Bl
				, self.symbol_table.current_frame()
				, current_block_num
				, current_address + 1
				, vec!(eval_block_num)
				, Token::new()
			),function_num
		);

		//	Now all of the machine instructions will go here
		self.make_block_current(eval_block_num, function_num);
	}
	pub fn gen_when(&mut self, when_expression : &Vec<Token>, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("CodeGen::GEN_WHEN: {}", token_list_text(&when_expression));}
		if self.cli.is_debug_bit(INSERT_DIAG_COMMENTS){
			self.add_machine_instruction(
				MachineInstruction::new(
					Opcode::Diag
					, OpcodeMode::NONE
					, self.symbol_table.current_frame()
					, 0
					, 0
					, Vec::new()
					, Token::make_string(&format!("=== gen_when === {}",when_expression[0].line_text))
				),function_num
			);
		}

		let eval_data_index = self.eval_data.len() - 1;

		self.eval_data[eval_data_index].when_counter += 1;

		let current_block_num = self.get_current_block_num(function_num);

		//	add a block to the block list.  save its number.  We probably can
		//	change this to current_block + 1 -- let's wait and see...
		let when_block_num = self.add_code_block(false, function_num);

		//	generate the when expression, it's at TOS-1
		self.gen_expression(when_expression, function_num);

		//	This is the random name created when eval was generated
		let target_var = Token::new2(
			TokenType::ID
			, self.eval_data[eval_data_index].target_var_name.clone()
			, 0
			, "ha ha".to_string()
			, TokenCategory::Factor
		);


		//  Get the Eval target (it's at TOS)
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Push
				, OpcodeMode::Var
				, self.symbol_table.current_frame()
				, self.eval_data[eval_data_index].target_detail.block_num
				, self.eval_data[eval_data_index].target_detail.index
				, Vec::new()
				, target_var.clone()
			),function_num
		);

		//	The top two operands are the expression and the target.  Compare
		//	them for equality. Leave the boolean result on the operand stack
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Eq
				, OpcodeMode::Var
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, Token::new()
			),function_num
		);	

		//	get the current instruction address
		let current_address = self.get_current_address(function_num);

		//	if the two operands were not equal. jump around
		//	the branch and link
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Jf
				, OpcodeMode::Jump
				, self.symbol_table.current_frame()
				, 0
				, current_address + 3   
				, Vec::new()
				, Token::new()
			),function_num
		);
		
		//	branch and link to the eval block
		//	current address + 1:  call the if block return to the next instruction
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Bl
				, OpcodeMode::Bl
				, self.symbol_table.current_frame()
				, current_block_num
				, current_address + 2
				, vec!(when_block_num)
				, Token::new()
			),function_num
		);

		//	And if there was a match, after executing the block
		//	do a block end which we jump around if the when expression
		//	didn't match the target.  The target of the jump will be a
		//	branch and link to an otherwise block (if there is one), otherwise it'll be
		//	1 instruction beyond the current clode bloc (but we don't know what that'll be
		//	yet)
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::BlockEnd
				, OpcodeMode::NONE
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, Token::new()
			),function_num
		);


		//	if this is the last when arm of the curren eval and
		//	we've seen an otherwise block somewhere in the arm list then
		//	emit code to call it here.  But if we haven't see it yet then
		//	we'll wait for it to appear in the statement list.  Also, if
		//	this is the last when arm and we've emitted the code to call the
		//	otherwise block, then we don't need the top element the eval_data
		//	block any more
		if self.eval_data[eval_data_index].when_counter == self.eval_data[eval_data_index].when_count{
			if self.eval_data[eval_data_index].otherwise_block_num != usize::MAX{
				self.add_machine_instruction(
					MachineInstruction::new(
						Opcode::Bl
						, OpcodeMode::Bl
						, self.symbol_table.current_frame()
						, current_block_num
						, current_address + 2
						, vec!(self.eval_data[eval_data_index].otherwise_block_num)
						, Token::new()
					),function_num
				);	
			}
			//	Also, if this is the last when arm then we remove the
			//	eval_data at the top of the stack
			self.eval_data.pop();
		}	

		//	Now any future instructions will go to the WHEN block
		//	which might not get executed but that's okay
		self.make_block_current(when_block_num, function_num);
	}


	pub fn gen_otherwise(&mut self, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("CodeGen::GEN_OTHERWISE");}
		if self.cli.is_debug_bit(INSERT_DIAG_COMMENTS){
			self.add_machine_instruction(
				MachineInstruction::new(
					Opcode::Diag
					, OpcodeMode::NONE
					, self.symbol_table.current_frame()
					, 0
					, 0
					, Vec::new()
					, Token::make_string(&format!("=== gen_otherwise ==="))
				),function_num
			);
		}

		let current_block_num = self.get_current_block_num(function_num);

		//	add a block to the block list.  save its number.  We probably can
		//	change this to current_block + 1 -- let's wait and see...
		let otherwise_block_num = self.add_code_block(false, function_num);

		//	get the current instruction address
		let current_address = self.get_current_address(function_num);

		//	emit the branch to the otherwise block
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Bl
				, OpcodeMode::Bl
				, self.symbol_table.current_frame()
				, current_block_num
				, current_address + 1
				, vec!(otherwise_block_num)
				, Token::new()
			),function_num
		);

		self.make_block_current(otherwise_block_num, function_num);

		//	Now, if the otherwise block occured somewhere else in the
		//	arm list we need to save its block address in the eval_data
		//	stack.   If the stack is empty we know this is the last arm
		//	in the eval arms
		if self.eval_data.len() == 0{
			return;
		}

		let eval_data_index = self.eval_data.len() - 1;
		self.eval_data[eval_data_index].otherwise_block_num = otherwise_block_num;

		// //  if this is the last arm of the evaluate then we can call the
		// //	otherwise block.  But if it's not, we wait until the last
		// //	arm (see gen_eval where we test the when count)

		// //	the eval_data stack is empty then this Otherwise must be
		// //	at the end of the when arm list and we can just emit the 
		// //	branch to the the block number.  If eval stack isn't emtpy
		// //	then is otherwise appeared somewhere else in the arm
		// //	list and we need to save it's block block numbe

		// let mut eval_data_index;
		// if 	self.eval_data.len() == 0{
		// 	eval_data_index = 0;
		// }else{

		// }
		// //	get the eval_data index
		// let eval_data_index = self.eval_data.len()-1;
		// if self.eval_data[eval_data_index].when_counter == self.eval_data[eval_data_index].when_count{
		// 	self.eval_data.pop();
		// }else{
		// }

		// //	Now all of the machine instructions will go here
	}

	pub fn gen_if(&mut self, condition : &Vec<Token>, has_else:bool, else_block_num : usize, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){
			println!("CodeGen::GEN_IF has_else={} else_block: {} cond: {}", has_else, else_block_num, token_list_text(&condition));
		}

		let current_block_num = self.get_current_block_num(function_num);

		//	add a block to the block list.  save its number.  We probably can
		//	change this to current_block + 1 -- let's wait and see...
		let if_block_num = self.add_code_block(false, function_num);
		
		//	generate the condition evaluation code
		self.gen_expression(condition, function_num);

		//	get an address that we'll be using as a return address (with some modifications)
		//  as well as the basis for jumping around
		let current_address = self.get_current_address(function_num);

		/*
				   <condition eval>
			n:     jf around if		target: n+3
			n+1:   call if			return to n+2
			if has_else
				n+2    j around else    target: n+4
			else
				n+2    j to continue	target: n+3
			
			if has_else
				n+3 call else		return to n+4
				n+4 continue
			else
				n+3 continue
		*/

		let jf_offset : usize;
		if has_else{
			jf_offset = 3;
		}else{
			jf_offset = 2;
		}

		//	current address:   jump around IF block
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Jf
				, OpcodeMode::Jump
				, self.symbol_table.current_frame()
				, 0
				, current_address + jf_offset
				, Vec::new()
				, Token::new()
			),function_num
		);

		//	current address + 1:  call the if block return to the next instruction
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Bl
				, OpcodeMode::Bl
				, self.symbol_table.current_frame()
				, current_block_num
				, current_address + 2
				, vec!(if_block_num)
				, Token::new()
			),function_num
		);

		
		//	When control returns to here, and there is an ELSE then jump around it
		if has_else{
			//  current addresss + 2
			self.add_machine_instruction(
				MachineInstruction::new(
					Opcode::J
					, OpcodeMode::Jump
					, self.symbol_table.current_frame()
					, 0
					, current_address + 4					// this will be modified at block end
					, Vec::new()
					, Token::new()
				),function_num
			);	
		}else{
			//  current address + 2 jump back into normal instruction stream
			self.add_machine_instruction(
				MachineInstruction::new(
					Opcode::J
					, OpcodeMode::Jump
					, self.symbol_table.current_frame()
					, 0
					, current_address + 3					// this will be modified at block end
					, Vec::new()
					, Token::new()
				),function_num
			);	
		}


		if has_else{
			self.add_machine_instruction(
				MachineInstruction::new(
					Opcode::Bl
					, OpcodeMode::Bl
					, self.symbol_table.current_frame()
					, current_block_num
					, current_address + 4
					, vec!(else_block_num)
					, Token::new()
				),function_num
			);
		}
		self.make_block_current(if_block_num, function_num);
	}


	//	This just ensures there is a code block to hold all of the
	//	instructions in the else block and makes the else block current
	//	so that all instructions go go there until BlockEnd
	pub fn gen_else(&mut self, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("CodeGen::GEN_ELSE");}
		let else_block = self.add_code_block(false, function_num);
		self.make_block_current(else_block, function_num);
	}

	/*
		n
				<condition eval>
		n+i     jf to end			target: n+i+3
		n+i+1:   call while			return to n+i+2
		n+i+2:   j (loop)			target: n

		where i = number of instructions generated for eval
	*/

	pub fn gen_while(&mut self, condition : &Vec<Token>, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("CodeGen::GEN_WHILE");}

		//	This is the block where the return instruction goes to
		let return_block_num = self.get_current_block_num(function_num);

		//	this is the target block for the while block
		let while_block_num = self.add_code_block(true, function_num);

		//	get the address of the expression evaluation instruction start
		let pre_condition_address = self.get_current_address(function_num);

		//	generate the condition evaluation code
		self.gen_expression(condition, function_num);

		//  get the address of the after the condition
		let post_condition_address = self.get_current_address(function_num);

		//	post_condition_address + 0:   if condition false, exit the loop
		//	and establish a break point
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Jf
				, OpcodeMode::Jump
				, self.symbol_table.current_frame()
				, 0
				, post_condition_address + 3
				, Vec::new()
				, Token::new()
			),function_num
		);

		self.break_address.push ((return_block_num, post_condition_address + 3));
		self.continue_address.push ((return_block_num, post_condition_address + 2));

		//	post_condition_address + 1 call the while loop, return to the next location
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Bl
				, OpcodeMode::Bl
				, self.symbol_table.current_frame()
				, return_block_num
				, post_condition_address + 2	// at this offset
				, vec!(while_block_num)			// bl target
				, Token::new()
			),function_num
		);
		
		//	post_condition_address + 2:   jump to pre_condition_address
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::J
				, OpcodeMode::Jump
				, self.symbol_table.current_frame()
				, 0
				, pre_condition_address
				, Vec::new()
				, Token::new()
			),function_num
		);

		self.make_block_current(while_block_num, function_num);
	}

	//	if the source is struct member, then it is actually an element of an array whose index
	//	we got from the symbol table
	fn gen_foreach_struct_member_source(&mut self, foreach_data : &ForeachData, function_num : usize){
		//	First, we need to get the address of the instantiated struct (e.g. "foo:")
		//	first we need to split this into the struct part and the member part
		let parts: Vec<&str> = foreach_data.foreach_source.token_value.split(':').collect();
		let mut struct_name = parts[0].to_string();
		struct_name.push(':');

		//	get the address of the instantiated struct
		let struct_detail = self.symbol_table.get_normal_address(&struct_name);

		//	push the instantiated struct onto the operand stack (this
		//	is the array that implements the instantiated struct)
		self.gen_expression_id_detail(&foreach_data.foreach_source, &struct_detail, function_num);

		//	next we need to get the element containing the array we want to
		//	iterate.  To do this we'll need the the member_number on the stack
		let member_entry = self.symbol_table.get_struct_member_entry(&foreach_data.foreach_source.token_value);

		//	Construct a token containing the index
		let mut index_token = foreach_data.foreach_source.clone();
		index_token.token_value = member_entry.member_index.to_string();
		index_token.token_category = TokenCategory::Factor;
		index_token.token_type = TokenType::INTEGER;
		
		//	Now push the index onto the stack
		self.gen_expression_scalar(&index_token, function_num);

		//  Emit the fetch indexed instruction
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::FetchIndexed
				, OpcodeMode::NONE
				, self.symbol_table.current_frame()
				, 0			
				, 0
				, Vec::new()
				, foreach_data.foreach_source.clone()
			),function_num
		);

		//  Now, theoretically, the top of the stack contains the array we
		//	are iterating.  Don't forget to pop the instatiated struct from the
		//	stack (which is just underneath the source array) when we're all
		//	done.
	}

	//	If the sorce is a normal variable, then we get it directly (as opposed to a struct
	//	member which we have to futz around to get it)
	fn gen_foreach_normal_source(&mut self, foreach_data : &ForeachData, function_num : usize){
		//	The source detail is an Option<SymbolTableEntryType> so we have to mess around
		//	to get the actual detail

		match &foreach_data.foreach_source_detail{
			None => abend!(format!("from gen_foreach_normal_source:  source detail was empty (None)")),
			Some(detail) => {
				if let SymbolTableEntryType::NormalSymbolEntry(entry) = detail{
					//	push the collection address onto the stack
					self.add_machine_instruction(
						MachineInstruction::new(
							Opcode::Push
							, OpcodeMode::Var
							, self.symbol_table.current_frame()
							, entry.block_num		// block num
							, entry.index			// address in block
							, Vec::new()
							, foreach_data.foreach_source.clone()
						),function_num
					);
				}else{
					abend!(format!("from gen_foreach_normal_source:  Soruce was souppose to be NORMAL but it wasn't"));
				}				
			}
		}
	}

	//	Add instructions which, when executed, generates true when the foreach
	//	loop reaches the end of the iteration.
	fn gen_foreach_condition(&mut self, foreach_data : &ForeachData, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("CodeGen::GEN_FOREACH_CONDITION");}

		//	Fetch the source (array) and push it onto the operand stack.
		if foreach_data.foreach_source.token_type == TokenType::QUALIFIED_ID{
			self.gen_foreach_struct_member_source(foreach_data, function_num);
		}else{
			self.gen_foreach_normal_source(foreach_data, function_num);
		}

		//	push the iteration counter address onto the stack
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Push
				, OpcodeMode::Var
				, self.symbol_table.current_frame()
				, foreach_data.foreach_iter_counter_detail.block_num		// block num
				, foreach_data.foreach_iter_counter_detail.index 			// address in block
				, Vec::new()
				, foreach_data.foreach_iter_counter.clone()
			),function_num
		);


		//	need a number token = 1
		let one = Token::new2(
			TokenType::INTEGER
			, "1".to_string()
			, 0
			, "ha ha".to_string()
			, TokenCategory::Factor
		);
	
		//	increment the iteration counter (it's already on the stack)
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Push
				, OpcodeMode::Lit
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, one.clone()
			),function_num
		);

		//	add the 1 to the iteration counter (the second instance of it)
		//	leaves result on the stack.
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Add
				, OpcodeMode::Var
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, Token::make_string("gen_foreach_condition")
			),function_num
		);

		//	and update the actual iteration counter (not the copy on the stack).  The
		//	update removes the update value so that only the unmodified instance
		//	of the iter counter is on the stack.
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Update
				, OpcodeMode::Update
				, self.symbol_table.current_frame()
				, foreach_data.foreach_iter_counter_detail.block_num
				, foreach_data.foreach_iter_counter_detail.index
				, Vec::new()
				, Token::new()
			),function_num
		);

		//	Get the iteration counter in preperation for the fetch indexed
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Push
				, OpcodeMode::Var
				, self.symbol_table.current_frame()
				, foreach_data.foreach_iter_counter_detail.block_num		// block num
				, foreach_data.foreach_iter_counter_detail.index			// address in block
				, Vec::new()
				, foreach_data.foreach_iter_counter.clone()
			),function_num
		);

		//	generate the FetchIndexed Instruction which leaves the result of the fetch
		//	on the stack
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::FetchIndexed
				, OpcodeMode::Internal				// this instruction generated internally
				, self.symbol_table.current_frame()
				, 0									// frame number of called function
				, 0
				, Vec::new()						//  NOTE:  was vec!(1) to restrict to array only
				, foreach_data.foreach_source.clone()
			),function_num
		);

		// the fetch_indexed instructions returns a CplVar which we want to
		// update the target with
		self.frames.frames_list.get_mut(function_num).unwrap().add_machine_instruction(
			MachineInstruction::new(
				Opcode::Update
				, OpcodeMode::Update
				, self.symbol_table.current_frame()
				, foreach_data.foreach_target_detail.block_num
				, foreach_data.foreach_target_detail.index
				, Vec::new()
				, foreach_data.foreach_target.clone()
			)
		);

		//  Since the update instruction removes the var that it uses to
		//  update, we have to fetch it again do we can get it's type
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Push
				, OpcodeMode::Var
				, self.symbol_table.current_frame()
				, foreach_data.foreach_target_detail.block_num		// block num
				, foreach_data.foreach_target_detail.index			// address in block
				, Vec::new()
				, foreach_data.foreach_target.clone()
			),function_num
		);

		//	construct a token that repsents the function call for Type.
		let get_type_function = Token::new2(
			TokenType::FUNCTION_CALL(1)
			, "Type".to_string()
			, foreach_data.foreach_source.line_number
			, foreach_data.foreach_source.line_text.clone()
			, TokenCategory::FunctionCall
		);


		let target_frame_info = self.get_function_frame_info(&get_type_function.token_value);
		let target_parameter_count = self.get_function_parameter_count(target_frame_info.0);

		//	now, we've got the result of the FetchIndexed instruction, on the stack
		//	we want get it's type on the stack via the Type function call
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::FunctionCall
				, OpcodeMode::Extern
				, self.symbol_table.current_frame()
				, target_frame_info.0			// frame number of called function
				, 0
				, vec!(target_parameter_count, 0)	// number of parameters declared, is_statement=false
				, get_type_function
			),function_num
		);

		//	now we've got the target type on the stack.  We need to see if it's
		//	a CplUndefined type and, if so, exit the loop, otherwise jump back to
		//	the beginning of the block.  Recall (if you can) that Type returns
		//	a CplString containing the name of the data type.  So we're effectively
		//	comparing strings.  Push the literal "CplUndefined" onto the stack.  First
		//	create a string token
		
		let cpl_undefined = Token::new2(
			TokenType::STRING
			, "CplUndefined".to_string()
			, foreach_data.foreach_source.line_number
			, foreach_data.foreach_source.line_text.clone()
			, TokenCategory::Factor
		);

		//	now add the instruction that pushes the "undefined" string onto the stack
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Push
				, OpcodeMode::Lit
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, cpl_undefined.clone()
			),function_num
		);	

	 	//	now add the comparison, which will compare the
		//	the top two element of stack and return:  -1, 0 or 1 depending on the textual
		//	comparison of the two strings.  Eq asks are these two things the same.
		//	the resut is either true or false
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Eq
				, OpcodeMode::Var
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, cpl_undefined.clone()
			),function_num
		);	
	}

	/*
		n
				<condition eval>
		n+i     jt to end			target: n+i+3  (if result of nextiter is undefined)
		n+i+1:   call while			return to n+i+2
		n+i+2:   j (loop)			target: n

		where i = number of instructions generated for eval.
	*/	
	pub fn gen_foreach(&mut self, target : &Token, source_type : &TokenType, source : &Vec<Token>, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("CodeGen::GEN_FOREACH target: {} source type: {} source: {}",target, source_type, source[0]);}
		
		//	Create a place for the foreach data in this foreach statement
		let mut foreach_data = ForeachData::new();
		
		//	the block to return to
		let return_block_num = self.get_current_block_num(function_num);
		//	add the target block
		let foreach_block_num = self.add_code_block(true, function_num);

		//	save the collection token
		foreach_data.foreach_source = source[0].clone();
		//	save the target token
		foreach_data.foreach_target = target.clone();

		//	get the collection (source) detail from the symbol table.  It might be a
		//	struct member or it might be a normal variable
		if foreach_data.foreach_source.token_type == TokenType::QUALIFIED_ID{
			let source_detail = self.symbol_table.get_struct_member_entry(&source[0].token_value);
			foreach_data.foreach_source_detail = Some(SymbolTableEntryType::StructMemberEntry(source_detail));
		}else{
			let source_detail = self.symbol_table.get_normal_address(&source[0].token_value);
			foreach_data.foreach_source_detail = Some(SymbolTableEntryType::NormalSymbolEntry(source_detail));
		}

		//	Create a target variable or use it if it alrelady exists
		match self.symbol_table.get_symbol_entry(&target.token_value){
			None		=> {
				//	add the symbol and return its detail
				foreach_data.foreach_target_detail = self.symbol_table.add_normal_symbol(&target.token_value);
				//	Generate the alloc for the new symbol
				self.gen_alloc(&target, foreach_data.foreach_target_detail.block_num, foreach_data.foreach_target_detail.index, function_num);
			},
			//	the symbol exists so just get its detail	
			Some (entry) 	=> {
				if let SymbolTableEntryType::NormalSymbolEntry(d) = entry{
					foreach_data.foreach_target_detail = d;
				}
			}
		}

		// Create a token representing the iteration counter.  It has a random
		// name and it is a "normal" var
		let iter_counter_name = self.get_random_id();
		foreach_data.foreach_iter_counter =
			Token::new2(
				TokenType::ID
				, iter_counter_name.clone()
				, target.line_number
				, target.line_text.clone()
				, TokenCategory::Factor
			);


		//	Add the iteration counter to the operand stack to make it a real
		//	variable
		//	if the symbol already exists, then it's an error:  this is a temporary variable
		match self.symbol_table.get_symbol_entry(&iter_counter_name){
			Some(_) => abend!(format!("Houston we have a problem.  gen_foreach trying to add iteration counter twice")),
			None => {
				//	add the symbol and return its detail
				foreach_data.foreach_iter_counter_detail = self.symbol_table.add_normal_symbol(&iter_counter_name);
				
				//	Generate the alloc for the new symbol
				self.gen_alloc(&foreach_data.foreach_iter_counter, foreach_data.foreach_iter_counter_detail.block_num, foreach_data.foreach_iter_counter_detail.index , function_num);

				let iter_value_0 = Token::new2(
					TokenType::INTEGER
					, "-1".to_string()
					, 0
					, "ha ha".to_string()
					, TokenCategory::Factor
				);
			
				//	Set the iteration counter to -1, push a -1 on to the stack.  This is because
				//	the foreach_condition increments the counter before it tries to fetch the
				//	item from the collection
				self.add_machine_instruction(
					MachineInstruction::new(
						Opcode::Push
						, OpcodeMode::Lit
						, self.symbol_table.current_frame()
						, 0
						, 0
						, Vec::new()
						, iter_value_0
					),function_num
				);

				//	Set the iteration counter to -1
				self.add_machine_instruction(
					MachineInstruction::new(
						Opcode::Update
						, OpcodeMode::Update
						, self.symbol_table.current_frame()
						, foreach_data.foreach_iter_counter_detail.block_num
						, foreach_data.foreach_iter_counter_detail.index
						, Vec::new()
						, foreach_data.foreach_iter_counter.clone()
					),function_num
				);
			},
		}
		//	Now, generate the condition evaluation instruction which leaves either true or
		//	false on the stack

		//	get the address of the expression evaluation instruction start
		let pre_condition_address = self.get_current_address(function_num);
		
		//	generate instruction that will compute the result of the condition
		self.gen_foreach_condition (&foreach_data, function_num);

		//	get the address after the condition computation
		let post_condition_address = self.get_current_address(function_num);

		//	post_condition_address + 0:   if condition true (meaning: next_iter returned undefineed), exit the loop
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Jt
				, OpcodeMode::Jump
				, self.symbol_table.current_frame()
				, 0
				, post_condition_address + 3
				, Vec::new()
				, Token::new()
			),function_num
		);

		self.break_address.push((return_block_num, post_condition_address + 3));
		self.continue_address.push((return_block_num, post_condition_address + 2));

		//	post_condition_address + 1 call the while loop, return to the next location
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Bl
				, OpcodeMode::Bl
				, self.symbol_table.current_frame()
				, return_block_num
				, post_condition_address + 2
				, vec!(foreach_block_num)		// target block
				, Token::new()
			),function_num
		);
		
		//	post_condition_address + 2:   jump to pre_condition_address (i.e. loop)
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::J
				, OpcodeMode::Jump
				, self.symbol_table.current_frame()
				, 0
				, pre_condition_address
				, Vec::new()
				, Token::new()
			),function_num
		);

		//	start adding instructions to the new foreach block
		self.make_block_current(foreach_block_num, function_num);
	}
}