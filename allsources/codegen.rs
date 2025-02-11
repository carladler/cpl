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
use literalmodel::*;
use macrolib::*;

use std::cell::RefCell;
use names::*;



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

	//	foreach source details
	foreach_source_block_num : usize,
	foreach_source_address : usize,

	// //	But the source might be a struct member containing an array so
	// //	it's type is "variable".  It might be normal or might be struct member  
	// foreach_source_detail : Option<SymbolTableEntryType>,
}

impl ForeachData{
	pub fn new() -> ForeachData{
		ForeachData{
			foreach_iter_counter : Token::new(),
			foreach_iter_counter_detail : NormalSymbolEntry::initialize(),
			foreach_target : Token::new(),
			foreach_target_detail : NormalSymbolEntry::initialize(),
			foreach_source: Token::new(),
			foreach_source_block_num : 0,
			foreach_source_address : 0,

			// foreach_source_detail : None,	
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

/**********************************************
****	Struct Instantiator Member Stack Entry
**********************************************/

//	As part of the struct instantiation code generation
//	a stack is used to keep track of which struct/substruct
//	we are in the process of emitting code for.  In the
//	instantiation loop, when a new substruct is encountered, the
//	information about the current substrtuct is pushed onto the
//	stack.  The new stack's member list is then processed.  When
//	the member list of the current substruct is exahusted the
//	previous entry is popped.  This continues until the
//	member list of the top-level struct is exhausted.
#[derive(Clone)]
struct StructMemberStackEntry{
	current_struct : Struct,
	current_member : usize,
	current_name_prefix : String,
}

impl StructMemberStackEntry{
	fn new (current_struct : Struct, current_member : usize, current_name_prefix : String) -> StructMemberStackEntry{
		StructMemberStackEntry{
			current_struct : current_struct,
			current_member : current_member,
			current_name_prefix : current_name_prefix.clone()
		}
	}

	//	Might need this, but maybe not
	fn _new2(entry : StructMemberStackEntry) -> StructMemberStackEntry{
		StructMemberStackEntry{
			current_struct : entry.current_struct.clone(),
			current_member : entry.current_member,
			current_name_prefix : entry.current_name_prefix.clone()
		}
	}
}

impl fmt::Display for StructMemberStackEntry{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"{} {} {}", self.current_struct.name, self.current_member, self.current_name_prefix)
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

	index_expression_comma_counter : usize,

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
	pub fn new(cli : & 'a CLI<'a>, names : & 'a RefCell<Names>) -> CodeGen<'a>{
		CodeGen{
			cli : cli,
			frames : FrameMap::new(),
			symbol_table : SymbolTable::new(cli, names),
			delimiter_counter : 0,
			collection_context : Vec::new(),
			block_begin_counter : 0,
			index_expression_comma_counter : 0,
			break_address : Vec::new(),
			continue_address : Vec::new(),
			//function_call_hold : Vec::new(),
			eval_data : Vec::new(),
			struct_list : Vec::new(),
			struct_map : HashMap::new(),
		}
	}

	pub fn dump_structs(&self){
		let mut header_index = 0;
		for s in &self.struct_list{
			s.dump_structs(header_index);
			header_index += 1;
		}
	}

	//	Add the declared structs to the local struct map.  So, why do
	//	use the local map instead of the symbol table which seems to make more
	//	sense?
	//
	//	Mostly it's becuase the "initializer" is an array of Tokens and the symbol
	//	table doesn't know what a Token is.  And I didn't want to teach it about
	//	Tokens (seems like Tokens ane symbols aren't related concepts).
	//
	//	We need a way of connecting an instantiated struct (as in foo = new struct) to
	//	the declared struct and from there to the struct's members.  The instantiated
	//	struct is treated as an array whose elements are its members (hence the requirement
	//	for a linkage to the struct's declaration).	
	pub fn add_structs_to_struct_map(&mut self, structs : &Vec<Struct>){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){
			eprintln!("CodeGen:add_structs_to_struct_map");
			for s in structs{
				let struct_name = s.name.clone();
				eprintln!("struct {}", struct_name);
				// for m in &s.members{
				// 	eprintln!("   {}:{}", struct_name, m.name);
				// }
			}
		}

		let mut index = 0;
		for s in structs{
			self.struct_map.insert(s.name.clone(), index);
			self.struct_list.push(s.clone());
			index += 1;
		}
	}

	pub fn add_global_literals(&mut self, global_literals : &Vec<LiteralStatement>){
		for global_literal in global_literals{
			let value_token = global_literal.literal_value[0].clone();

			let entry : LiteralEntry;

			match value_token.token_type{
				TokenType::FLOAT | TokenType::INTEGER => {
					entry = LiteralEntry::new(LiteralType::LiteralNumber(LiteralNumber::new(value_token.token_value.parse::<f64>().unwrap())));
				}
				TokenType::STRING => {
					entry = LiteralEntry::new(LiteralType::LiteralString(LiteralString::new(&value_token.token_value)));
				},
				TokenType::BOOL => {
					if value_token.token_value == "false"{
						entry = LiteralEntry::new(LiteralType::LiteralBool(LiteralBool::new(false)));
					}else{
						entry = LiteralEntry::new(LiteralType::LiteralBool(LiteralBool::new(true)));
					}
				},
				_=> panic!("from SymbolTable.add_literal:  {} is not supported", value_token),
			}
			self.symbol_table.add_global_symbol(&global_literal.literal_id.token_value, &SymbolTableEntryType::LiteralEntry(entry));
		}
	}

	fn get_random_id(&self) -> String{
		let temp : u16 = random();
		let rtn = format!("$temp{}", temp);
		rtn.clone()
	}

	fn error_abend (&self, error_text : &str){
		eprintln!("CPL abend due to code generation error: {}", error_text);
		std::process::exit(1);
	}


	//	Get the current address of the current code block (i.e. the address counter)
	fn get_current_address (&self, function_num : usize) -> usize{
		self.frames.frames_list[function_num].get_current_address()
	}

	fn get_current_block_num(&self, function_num : usize) -> usize{
		self.frames.frames_list[function_num].get_current_block_num()
	}


	fn add_machine_instruction(&mut self, instruction : MachineInstruction, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN_ADD_INSTRUCTION){
			eprintln!("add_machine_instruction: {}:{}", self.frames.frames_list.get_mut(function_num).unwrap().get_address_counter(), instruction);
		}
		self.frames.frames_list.get_mut(function_num).unwrap().add_machine_instruction(instruction);
	}

	fn make_block_current(&mut self, block_num: usize, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    make_block_current: {} block counter: {}", block_num, self.block_begin_counter);}
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
				, 0
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

		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    add_code_block: added {}", block_num);}

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
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("CodeGen::ADD_FUNCTION_TO_FRAME function name={}", name);}

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

	//	TODO:  this needs to be amended after we get indices working
	fn add_struct_parameter_to_symbol_table(&mut self, parameter_name : &String, struct_name : &String){

		//eprintln!("=============== add_struct_parameter_to_symbol_table parameter_name={} struct_name={}",parameter_name,struct_name);
		self.symbol_table.add_normal_symbol(parameter_name);

		//  Get the index of this struct in the struct map
		let struct_index_option = self.struct_map.get(struct_name);
		if struct_index_option == None{
			abend!(format!("from add_struct_parameter_to_symbol_table:  Struct {} has not been defined", struct_name));
		}
		let struct_index : usize = *struct_index_option.unwrap();

		//	The index field of this symbol_detail points us to an entry in
		//	the struct_list attribute of codegen.
		let local_struct = self.struct_list[struct_index].clone();
		let _local_members = local_struct.members.clone();

		//	************  NOTE THIS IS WRONG...  See TODO above
		let mut _member_index = vec!(0);	

		//	Cycle through the members adding them to the symbol table 
		// for member in &local_members.members{
		// 	match member {
		// 		StructMemberType::Field(field) => {
		// 			//	Add this local reference to the symbol table.
		// 			let member_ref = format!("{}:{}", parameter_name, field.name);

		// 			if self.cli.is_debug_bit(TRACE_CODE_GEN){
		// 				eprintln!("    CodeGen:add_struct_parameter_to_symbol_table:adding {} ({},{}) to symbol table",member_ref, struct_index, member_index);
		// 			}
		// 			self.symbol_table.add_struct_member(member_ref, struct_index, member_index);
		// 		}
		// 		_=> {}
		// 	}
		// 	member_index += 1;
		// }

		//self.symbol_table.symbol_table_dump_diag("end of add_struct_parameter_to_symbol_table");
	}


	//	Emit instructions to start a function.
	pub fn gen_function(&mut self, name : &String, function_parameters : &Vec<String>, function_entry_flag : bool, function_num : usize, cl_args : &Vec<String>){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("CodeGen::GEN_FUNCTION function name={} num={}", name, function_num);}

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
						, symbol_detail.interner
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
						, symbol_detail.interner
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
		let detail = self.symbol_table.add_normal_symbol(&function_parameters[0]);
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
				, detail.interner
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
					, 0
					, token.clone()
				),function_num
			);	

			//	And, now we can update the array.  This is a direct update (as
			//	opposed to via a VarRef)
			self.add_machine_instruction(
				MachineInstruction::new(
					Opcode::Append
					, OpcodeMode::NONE		//push tos onto array at tos
					, self.symbol_table.current_frame()
					, 0
					, 0
					, Vec::new()
					, 0
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
			eprintln!("CodeGen::GEN_BLOCK_END count {}",self.block_begin_counter);
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
				, 0
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
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    gen_expression_factor: {}", token);}

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
		// if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    gen_expression_struct_member: {}", token);}

		//	first we need to split this into the struct part and the member part
		let parts: Vec<&str> = token.token_value.split(':').collect();
		let struct_name = parts[0].to_string();

		//	get the address of the instantiated struct (which is really an array)
		//	The payload in the symbol table is a normal entry.  This will be the
		//	base array.
		let struct_detail = self.symbol_table.get_normal_address(&struct_name);

		//	push a reference to the instantiated struct onto the operand stack
		self.gen_expression_struct(token, struct_detail.block_num, struct_detail.index, function_num);

		//	if this is a struct reference (i.e. <name>: without any member), as oppose to a struct member reference,
		//	I think we just push the struct (i.e. the array) onto the operand stack as opposed to
		//	pushing a specific member onto the stack.
		if parts[1].len() == 0{
			if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    gen_expression_struct_member: {}", token);}
			let mut ref_token = token.clone();
			ref_token.token_type = TokenType::STRUCT;
			self.gen_expression_struct(&ref_token, struct_detail.block_num, struct_detail.index, function_num);
			return;
		}
		
		/*
		When we are dealing with a qualified identifier (e.g. foo:bar) we need to navigate
		to an element in the underying array.  Each element is described by a "path"
		list:  a list of indexes.  The path is, essentially, an index into a n-dimensional
		array.  That is elements containing arrays containing elements which contain arrays
		and so on.

		We search the symbol table for the fully qualified identifier (e.g. foo:bar:baz:member)
		the detail returned will include a "path".  The path will consist of (in this case)
		3 numbers:  index of bar in foo, index of baz in bar and index of member in baz.

		There is, at this point already a reference to the instantiated struct variable (foo)
		on the stack.  The next instructions need to be:

			push index of member
			push index of baz
			push index of bar
			fetch_indexed (with the number of indexes in the instruction)
		
		Notice that the indices were pushed onto the stack in reverse order so that the
		lowest dimension is first.  The fetch indexed instruction was designed to
		accomidate expressions like foo[1,2,3] which, as it turns out is exactly what
		we needed to deal with multi-level structs (in reverse order).  See exec_fetch_indexed.
	
		*/

		//	get the entry for the member we're after
		let member_entry = self.symbol_table.get_struct_member_entry(&token.token_value);

		//	Construct a token to hold the index
		let mut index_token = token.clone();
		index_token.token_category = TokenCategory::Factor;
		index_token.token_type = TokenType::INTEGER;

		//	Now build instructions to push the indices onto the stack.  Notice
		//	the first index will end up at the top of the stack

		//let mut i : i32 = (member_entry.member_index.len()-1) as i32;
		let mut i=0;
		while i < member_entry.member_index.len(){
			index_token.token_value = member_entry.member_index[i as usize].to_string();
			//	Now push the index onto the stack
			self.gen_expression_scalar(&index_token, function_num);
			i+=1;
		}

		if self.cli.is_debug_bit(TRACE_CODE_GEN){
			eprintln!("    gen_expression_struct_member {} path length={} path={:?}", token.token_value, member_entry.member_index.len(), member_entry.member_index);
		}
		//  Emit the fetch indexed instruction
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::FetchIndexed
				, OpcodeMode::NONE
				, self.symbol_table.current_frame()
				, 0			
				, 0
				, vec!(member_entry.member_index.len())
				, 0
				, token.clone()
			),function_num
		);
	}


	//	If the token is either an ID or IDADDR (e.g. &x), grab it and push it onto the
	//	operand stack.  For example a = b will generate this as will foo(&x).  The mode
	//	for the former is "Var" and "VarRef" for the latter.
	fn gen_expression_id_detail(&mut self, token : &Token, symbol_detail : &NormalSymbolEntry, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    gen_expression_id_detail: {} {}", token, symbol_detail);}
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
				, symbol_detail.interner
				, token.clone()
			),function_num
		);
	}

	//	If the ID was the name of an instantiated struct, we want to push a VarRef
	//	pointing to it onto the stack.
	fn gen_expression_struct(&mut self, token : &Token, symbol_block_num : usize, symbol_index : usize, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    gen_expression_struct: {}", token);}

		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Push
				, OpcodeMode::VarRef
				, self.symbol_table.current_frame()
				, symbol_block_num			// block num
				, symbol_index 				// address in block
				, Vec::new()
				, 0
				, token.clone()
			),function_num
		);
	}

	fn gen_expression_id(&mut self, token : &Token, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    gen_expression_id: {}", token);}

		//	let's look at the id (token) in the symbol table because it might be either
		//	a normal symbol or a struct

		//self.symbol_table.print_entry_type(&token.token_value);

		let entry = self.symbol_table.get_symbol_entry(&token.token_value);
	
		match entry {
			None => abend!(&format!("From gen_expression_id: {} Not in symbol table",token.token_value)),
			Some(ref e) => match e{
				SymbolTableEntryType::NormalSymbolEntry(normal_symbol_detail) => {
					self.gen_expression_id_detail(token, &normal_symbol_detail, function_num);
				}

				SymbolTableEntryType::StructMemberEntry(_) => {
					self.gen_expression_struct_member(&token, function_num);
				}

				SymbolTableEntryType::StructEntry(struct_detail) => {
					self.gen_expression_struct(token, struct_detail.block_num, struct_detail.index, function_num);
				}

				SymbolTableEntryType::StructChildEntry (_struct_detail) => {
				}

				//	If it turns out the id is actually a literal, just push it's value onto
				//	the stack
				SymbolTableEntryType::LiteralEntry(literal_detail) => {
					self.gen_expression_literal(token, &literal_detail.literal_type, function_num);
				}
			}
		}
	}

	fn gen_expression_literal(&mut self, token : &Token, literal_type : &LiteralType, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    gen_expression_literal: {}:{}", token, literal_type);}

		let mut lit_token = token.clone();
		lit_token.token_category = TokenCategory::Factor;
		

		match literal_type{
			LiteralType::LiteralNumber(ref v) 	=>{
				lit_token.token_type = TokenType::FLOAT;
				lit_token.token_value = v.literal_number.to_string();
			}
			LiteralType::LiteralString(ref v) 	=>{
				lit_token.token_type = TokenType::STRING;
				lit_token.token_value = v.literal_string.clone();
			}
			LiteralType::LiteralBool(ref v) 	=>{
				lit_token.token_type = TokenType::BOOL;
				lit_token.token_value = v.literal_bool.to_string();
			}
			_ => {
				panic! ("from gen_expression_literal_detail: Array and Dictionary Literals not supported yet");
			}
		}

		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Push
				, OpcodeMode::Lit
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, 0
				, lit_token.clone()
			),function_num
		);
	}

	fn gen_expression_scalar(&mut self, token : &Token, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    gen_expression_scalar: {}", token);}

		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Push
				, OpcodeMode::Lit
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, 0
				, token.clone()
			),function_num
		);		
	}

	fn gen_expression_function_call(&mut self, token : &Token, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    gen_expression_function_call: {}", token);}

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
			opcode_mode = OpcodeMode::Builtin;
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
			, 0
			, token.clone()						// function name
		);

		self.add_machine_instruction(fc, function_num);
	}

	//	When the token in the expression is a "list separator" type comma, we
	//	are constructing an array via a literal array (e.g. [1,2,3]) and this
	//	causes the update of the array.  The array is at the top of the stack
	//	And will never be "popped".
	fn gen_expression_comma(&mut self, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    gen_expression_comma");}

		if self.collection_context.len() > 0{
			//	only update if context is array.  Dictionary updates are triggered by RDict tokens
			if let CollectionContext::Array = self.collection_context.last().unwrap(){
				//	update the array variable at the top of the stack
				self.add_machine_instruction(
					MachineInstruction::new(
						Opcode::Append
						, OpcodeMode::NONE
						, self.symbol_table.current_frame()
						, 0
						, 0
						, Vec::new()
						, 0
						, Token::new()
					),function_num
				);							
			}
		}
	}

	fn gen_expression_lbracket(&mut self, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    gen_expression_lbracket");}
		
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
				, 0
				, Token::new()
			),function_num
		);							
	}


	//	Evidently we only see a "]" at the end of a literal array expression  If
	//	we see a Rindex ("]") then it'll be at the end of an index expression
	//	We are creating a new array to be populated via a literal expression.
	//	it'll never be popped.
	fn gen_expression_rbracket(&mut self, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    gen_expression_rbracket: delimiters={}", self.delimiter_counter);}

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
				Opcode::Append
				, OpcodeMode::NONE
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, 0
				, Token::new()
			),function_num
		);	
	}

	fn gen_expression_rindex(&mut self, token : &Token, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    gen_expression_rindex: {}", token);}
		//  At this point TOS is an index and TOS-1 is an array.  The rindex (i.e. "]"
		//	triggers the fetch_indexed exec function).

		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::FetchIndexed
				, OpcodeMode::NONE
				, self.symbol_table.current_frame()
				, 0			
				, 0
				, vec!(self.index_expression_comma_counter)
				, 0
				, token.clone()
			),function_num
		);
	}

	//	This is the bginning of a dictionary literal.  Create a new empty dictionary
	//	at the top of the stack
	fn gen_expression_ldict(&mut self, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    gen_expression_ldict");}
		self.collection_context.push(CollectionContext::Dict);

		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::PushNewCollection
				, OpcodeMode::Dict
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, 0
				, Token::new()
			),function_num
		);
	}

	//	This is the end of a label_value pair, add the key and value to
	//	the dictionary.  This is triggered via the construction of dictionary
	//	via a literal dictionary expression
	fn gen_expression_rdict_kv(&mut self, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    gen_expression_rdict_kv");}

		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Insert
				, OpcodeMode::NONE
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, 0
				, Token::new()
			),function_num
		);

	}
	
	//	This is the end of a dictionary literal
	fn gen_expression_rdict(&mut self, _frame_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    gen_expression_rdict");}

		self.collection_context.pop();
	}

	fn gen_expression(&mut self, expression_list : &Vec<Token>, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("CodeGen.gen_expression {}",token_list_text(expression_list));}

		for t in expression_list{
			if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    gen_expression {}", t);}

			//	This is a special case (HACK!!!!) that happens when an empty array is getting
			//	initialized
			if t.token_type == TokenType::NEW_COLLECTION{
				let mode : OpcodeMode;
				if t.token_category == TokenCategory::LBracket{
					mode = OpcodeMode::Array;
				}else if t.token_category == TokenCategory::LBrace{
					mode = OpcodeMode::Dict;
				}else{
					mode = OpcodeMode::NONE;
				}
				self.add_machine_instruction(MachineInstruction::new(
					Opcode::PushNewCollection
					, mode
					, self.symbol_table.current_frame()
					, 0
					, 0
					, Vec::new()
					, 0
					, t.clone()),function_num
				);
				continue;
			}else if t.token_type == TokenType::LENGTH_OF{
				//	And this hack pushes the lengthof instruction on the
				//	stack
				self.add_machine_instruction(MachineInstruction::new(
					Opcode::LengthOf
					, OpcodeMode::Var
					, self.symbol_table.current_frame()
					, 0
					, 0
					, Vec::new()
					, 0
					, t.clone()),function_num
				);
				continue;
			}

			match t.token_category{
				TokenCategory::Factor => self.gen_expression_factor(&t, function_num),
				TokenCategory::IndexedId => {
					self.index_expression_comma_counter = 0;
					self.gen_expression_factor(&t, function_num);
				}

				TokenCategory::Comma => self.index_expression_comma_counter += 1,
				
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
						, 0
						, t.clone()),function_num
					);
				},
			}
		}
	}

	pub fn gen_loop(&mut self, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("gen_loop");}

		//	This is the block where the return instruction goes to
		let return_block_num = self.get_current_block_num(function_num);

		//	this is the target block for the while block
		let loop_block_num = self.add_code_block(true, function_num);

		//	Mark the current address
		let pre_condition_address = self.get_current_address(function_num);

		// self.break_address.push ((return_block_num, pre_condition_address + 2));
		self.continue_address.push ((return_block_num, pre_condition_address + 1));

		//	post_condition_address + 1 call the while loop, return to the next location
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Bl
				, OpcodeMode::Bl
				, self.symbol_table.current_frame()
				, return_block_num
				, pre_condition_address + 1		// at this offset
				, vec!(1,loop_block_num, return_block_num, pre_condition_address + 2)
				, 0
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
				, 0
				, Token::new()
			),function_num
		);

		self.make_block_current(loop_block_num, function_num);
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
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("CodeGen.gen_simple: {} {}",token.token_value, token_list_text(expression_list));}
		
		//	If the expression list is empty then the statement better be either BREAK or CONTINUE
		if expression_list.is_empty() 
			&& token.token_type != TokenType::BREAK
			&& token.token_type != TokenType::CONTINUE
			&& token.token_type != TokenType::EXIT {
			abend!(format!("From gen_simple: expected BREAK, CONTINUE or EXIT but got: {}", token.token_type));
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
				self.add_machine_instruction(
					MachineInstruction::new(
						  Opcode::Break
						, OpcodeMode::NONE
						, self.symbol_table.current_frame()
						, 0
						, 0
						, vec!(expression_list.len())
						, 0
						, token.clone()
					),function_num
				);	
			},

			TokenType::CONTINUE => {
				//let continue_address = self.continue_address.last().unwrap();
				self.add_machine_instruction(
					MachineInstruction::new(
						  Opcode::Continue
						, OpcodeMode::NONE
						, self.symbol_table.current_frame()
						, 0 //continue_address.0
						, 0 //continue_address.1
						, vec!(expression_list.len())
						, 0
						, token.clone()
					),function_num
				);	
			}

			TokenType::EXIT => {
				//let continue_address = self.continue_address.last().unwrap();
				self.add_machine_instruction(
					MachineInstruction::new(
						  Opcode::Exit
						, OpcodeMode::NONE
						, self.symbol_table.current_frame()
						, 0 //continue_address.0
						, 0 //continue_address.1
						, vec!(expression_list.len())
						, 0
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
					, 0
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
			TokenType::ASG_EQ 			=> Opcode::Update,		// always means update scalar

			_=> abend!(format!("From op_to_opcde:  {} is an unknown assignment operator", op.token_type)),
		}		
	}

	fn gen_struct_instantiate_push_uninitialized(&mut self, name : &str, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){
			eprintln!("    gen_struct_instantiate_push_uninitialized target={}", name);
		}
		//	Add a CplUninitialized to the array
		let mut uninit = Token::new();
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
				, 0
				, uninit.clone()
			),function_num
		);
	}



	fn gen_struct_field(&mut self, field : &Field, name_prefix : &str, member_index : &Vec<usize>, struct_index : usize, function_num : usize){
		let member_ref = format!("{}:{}", name_prefix, field.name);

		if self.cli.is_debug_bit(TRACE_CODE_GEN){
			eprintln!("    gen_struct_field:adding {} {} {:?} to symbol table",member_ref, struct_index, member_index);
		}
		//eprintln!("=======CodeGen:gen_struct_field:adding {} struct index={} member index={:?} to symbol table",member_ref, struct_index, member_index);

		//	If the member of the struct has an initializer, emit those instructions as well.
		if field.initializer.len() == 0{
			self.gen_struct_instantiate_push_uninitialized(&member_ref, function_num);
		}else{
			//	if the initializer is not empty, it is a list of tokens that
			//	we can pass directly to gen_expression which will emit instructions
			//	to compute the initailizer for this member.  Typically (I think always) this
			//	will be a literal factor (e.g. a string, number or boolean).
			self.gen_expression(&field.initializer, function_num)
		}

		//	Add this local reference to the symbol table.  Note that details for
		//	a struct member contains the index of the member in the instatiated
		//	struct.  And, since there isn't an "address" apart from the index
		//	the entry does not take up any space in the operand stack

		let member_detail = self.symbol_table.add_struct_member(member_ref, struct_index, member_index);

		//	Push the member onto the array at tos
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Append
				, OpcodeMode::NONE
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, member_detail.interner
				, Token::new()
			),function_num
		);
	}			

	pub fn gen_struct_instantiate(&mut self, instantiated_struct : &Token, struct_name : &Token, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){
			eprintln!("CodeGen:GEN_STRUCT_INSTANTIATE target={} struct={}", instantiated_struct.token_value, struct_name.token_value);
		}

		// eprintln!("======= CodeGen:gen_struct_instantiate target={} struct={}", instantiated_struct.token_value, struct_name.token_value);

		// self.dump_structs();

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
		//
		//  If the member is a child it goes into the symbol table as <target>:<child> with
		//	and entry type of StructChildEntry.
		let instantiated_struct_symbol = format!("{}",instantiated_struct.token_value);
		let struct_detail = self.symbol_table.add_normal_symbol(&instantiated_struct_symbol);

		//	and allocate an undefined variable
		self.gen_alloc(&instantiated_struct, struct_detail.block_num, struct_detail.index, struct_detail.interner, function_num);

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
				, 0
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


		//	The following loop is in lieu of recursion which (my head is about to explode) doesn't
		//	work.

		//	first create a variable that holds a reference to the struct.  This will be the
		//	current struct.  The loop always is iterating the members of some struct or other.
		//	initially it's the top-level struct.  Then when we get to a substruct, we push
		//	the current struct onto a stack, replace it with the struct to iterate.  If we run into
		//	other substruct then do the push/replace two-step again.  If we get to the end
		//	pop the struct and keep iterating from where we left off (so we have to push both
		//	the struct and the current member index)

		//	We need a local copy of the struct list.  Otherwise, we run into double immutable
		//	access of self (arrrrgghhhhh!!!!)
		let struct_list = self.struct_list.clone();
		let mut member_stack : Vec<StructMemberStackEntry> = Vec::new();

		//	start with the top level struct.  The entry is: the top level struct, the first member and the name of the instantatiator
		let mut member_stack_entry = StructMemberStackEntry::new(
				  struct_list.get(struct_index).unwrap().clone()
				, 0
				, instantiated_struct.token_value.clone()
		);

		//	This will be the member_indices vector.  Every time we move to a new level, we update the last
		//	element of this vector with current_member.  Whenever we move up a level we pop the the last entry.
		//	This vector is added to any field added to the symbol table
		let mut member_indices : Vec<usize> = vec!(member_stack_entry.current_member);

		loop{
			while member_stack_entry.current_member < member_stack_entry.current_struct.members.members.len(){
				match member_stack_entry.current_struct.members.members.get(member_stack_entry.current_member).unwrap(){
					StructMemberType::Field (ref field) =>{
						//eprintln!("======= Field: {}:{} member={} path={:?}",member_stack_entry.current_name_prefix, field.name, member_stack_entry.current_member, member_indices);
						self.gen_struct_field(field, &member_stack_entry.current_name_prefix, &member_indices, struct_index, function_num);

						//	And increment the index at the top of this stack
						member_stack_entry.current_member += 1;
						member_indices.pop().unwrap();
						member_indices.push(member_stack_entry.current_member);
					}
					StructMemberType::Substruct(substruct) => {
						//eprintln!("======= {} {} path={:?}", substruct, member_stack_entry.current_member, member_indices);
						//	save the struct we're working on (with the index pointing at the next one)
						//	which actually might not be there, but that's okay because we test the member_stack_entry.current_member
						//	against the length of this struct's member list

						member_stack.push(StructMemberStackEntry::new(
								  member_stack_entry.current_struct.clone()
								, member_stack_entry.current_member + 1
								, member_stack_entry.current_name_prefix.clone())
						);


						//	When we encounter a substruct, we need to create another
						//	array in the operand stack which will be populated via
						//	any subsequent fields.  then, when we reach the end of this struct's
						//	members (which may be a while since a member can be another substruct)
						//	we add this array to the array just below it (which, again, may be
						//	an array holding the fields of substrct -- and so on)

						self.add_machine_instruction(
							MachineInstruction::new(
								Opcode::PushNewCollection
								, OpcodeMode::Array
								, self.symbol_table.current_frame()
								, 0
								, 0
								, Vec::new()
								, 0
								, Token::new()
							),function_num
						);
						
						//	Starting a new member list so the index at the top of the path list is 0
						member_indices.push(0);

						//	set the current_struct to where we think it is and the new prefix
						member_stack_entry.current_struct = struct_list.get(substruct.id).unwrap().clone();
						member_stack_entry.current_member = 0;
						member_stack_entry.current_name_prefix = format!("{}:{}", member_stack_entry.current_name_prefix, member_stack_entry.current_struct.name);
						//eprintln!("======= Now working on {}", member_stack_entry.current_struct.name);
					}
				}
			}

			//	When we fall out of the foreach loop, we're done with fields in the current
			//	struct / substruct.

			//	if we're done with the top-level struct then break
			if member_stack.len() == 0{
				break;
			}


			//	Now add the substruct whose members we just finished processing to the struct
			//	just below it in the operand stack.  Note that the "path" list is the last
			//	instance of it before we fell out of the loop.
			let member_ref = member_stack_entry.current_name_prefix.clone();
			let member_detail = self.symbol_table.add_struct_member(member_ref, struct_index, &member_indices);

			//	Push the substruct array onto the array at the top of the stack
			self.add_machine_instruction(
				MachineInstruction::new(
					Opcode::Append
					, OpcodeMode::NONE
					, self.symbol_table.current_frame()
					, 0
					, 0
					, Vec::new()
					, member_detail.interner
					, Token::new()
				),function_num
			);


			//	Now get the struct at the top of the member_stack and
			//	finish up its members
			member_stack_entry = member_stack.pop().unwrap();
			
			//	And remove the entry in the path for the substruct we just finished
			member_indices.pop();

			//	And update the path entry at the top of the path stack
			member_indices.pop();
			member_indices.push(member_stack_entry.current_member);
			
			//eprintln!("======= Returning to: {} {:?}", member_stack_entry.current_struct.name, member_indices);
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
				, struct_detail.interner
				, instantiated_struct.clone()
			),function_num
		);

		//self.symbol_table.symbol_table_dump_diag("======= symbols at end of struct_instantiate");
	}

	fn _gen_inc_dec_collection(&mut self, _target : &Token, _op : &Token, _target_index_expression : &Vec<Token>, _expression_list : &Vec<Token>, _function_num : usize){
		panic!("Conversion of collection[index] += 1 to inc not implemented yet")
	}

	fn get_address_scalar(&mut self, target : &Token) -> (usize,usize){
		let detail = self.symbol_table.get_normal_address(&target.token_value);
		return (detail.block_num,detail.index);
	}
	
	//	Here we are converting the x+=1 to inc and x-=1 to dec.  We'll return false if we haven't implemented
	//	this conversion for a particular target type (i.e. we'll implement it first for scalars as
	//	this is probably the most highly used statement).
	fn gen_inc_dec(&mut self, target : &Token, op : &Token, target_index_expression : &Vec<Token>, _expresssion_list : &Vec<Token>, function_num : usize) -> bool{		
		//	I hope this actually improves performance for While index<value{...} loops because
		//	it's ugly

		let block_num : usize;
		let index : usize;

		//	And, if this is an assignment to a struct member or scalar, get the target's
		//	address.  But if it's an assignment to a collection member it's not
		//	as simple as getting the address of the target because the address of the
		//	target will be determined by what's at the top of the stack (i.e. the index
		//	expression)
		if target.token_type == TokenType::QUALIFIED_ID{
			return false;
		}else if target_index_expression.len() == 0{
			(block_num, index) = self.get_address_scalar(target);
		}else{
			return false;
		}

		if op.token_type == TokenType::ASG_ADD_EQ{
			self.add_machine_instruction(
				MachineInstruction::new(
					Opcode::Inc
					, OpcodeMode::Var
					, self.symbol_table.current_frame()
					, block_num		// block num
					, index 			// address in block
					, Vec::new()
					, 0
					, target.clone()
				),function_num
			);
		}else{
			self.add_machine_instruction(
				MachineInstruction::new(
					Opcode::Dec
					, OpcodeMode::Var
					, self.symbol_table.current_frame()
					, block_num		// block num
					, index 			// address in block
					, Vec::new()
					, 0
					, target.clone()
				),function_num
			);
		}

		return true;
	}

	//	Assignment to a struct member has to look like assignment to an
	//	an element of the array.  We'll need:  the address of the instantiated struct,
	//	an index token that we have to create and an expression vector whose first element
	//	is the index token.  the update is in situ (i.e. at a specified location in the stack)
	
	//	TODO:  This code is remarkably similar to the gen_assignment_to_collection that we should be able
	//	reduce the duplication.

	//	TODO:  We need as generalized approach to updating an lvalue which is a n-dimensional array
	//	to deal with both:  "foo:bar:zot:member = 5" and "foo[1,2,3] = 5".  At the moment this function
	//	and gen_assignment_to_collection only support a single index.
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

		//	NOTE TODO above... for now we only use the index into the top
		//  level struct
		index_token.token_value = member_entry.member_index[0].to_string();

		//	Add the instruction push the index on to the operand stack
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Push
				, OpcodeMode::Lit
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, 0
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
				, detail.interner
				, target.clone()
			),function_num
		);
	
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("CodeGen:GEN_ASSIGNMENT_TO_STRUCT_MEMBER target={}", target.token_value);}
	}

	fn gen_assignment_to_scalar(&mut self, target : &Token, op : &Token, expression_list : &Vec<Token>, function_num : usize){		
		let detail : NormalSymbolEntry;
		
		match self.symbol_table.get_symbol_entry(&target.token_value){
			None		=> {
				//	add the symbol and return its detail
				detail = self.symbol_table.add_normal_symbol(&target.token_value);

				//	Generate the alloc for the new symbol
				self.gen_alloc(&target, detail.block_num, detail.index, detail.interner, function_num);
			},
			//	the symbol exists so just get its detail	
			Some (entry) 	=> {
				match entry{
					SymbolTableEntryType::NormalSymbolEntry(d) => detail = d,

					//	If the variable was used previously as a struct instantiation then we 
					//	can use its slot in the operand stack but need to change the detail
					//	type from Struct to normal and update the detail values
					SymbolTableEntryType::StructEntry(m) => {
						detail = NormalSymbolEntry::new(m.block_num, m.index, m.interner);
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
				, detail.interner
				, target.clone()
			),function_num
		);

		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    gen_assignment_to_scalar: target={} detail={}", target.token_value, detail);}
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
				, detail.interner
				, target.clone()
			),function_num
		);

		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("    gen_assignment_to_collection target={} detail={}", target.token_value, detail);}
	}

	//	The syntax is:
	//
	//		literal <id> = <literal>
	//
	//	The scope of a literal depends on where it was encountered.  If it
	//	encountered outside of a function then it's scope is global.  If encountered
	//	inside a block, then it's scope is the block.
	//
	//	The "=" is sugar and can be omitted
	//
	//	<literal> may not be an Id:  it must be a number, string or bool (and possibly an array or dict literal)
	//
	//	target is the name of the literal
	//	exoression_list is the value (note that if an array or dictionary literal is used all of the components must
	//	be values (numbers, strings, etc))
	//
	//	if function_num == None then the literal is global
	// pub fn gen_assignment_to_literal (&mut self, target : &Token, op : &Token, expression_list : &Vec<Token>, function_num : Option<usize>){
	// }

	pub fn gen_assignment(&mut self, target : &Token, op : &Token, target_index_expression : &Vec<Token>,expression_list : &Vec<Token>, function_num : usize){		
		//	Here we test for x+=1 or x-=1 which we convert to x++ or x-- respectively
		if op.token_type == TokenType::ASG_ADD_EQ || op.token_type == TokenType::ASG_SUB_EQ{
			if expression_list.len() == 1 {
				if expression_list[0].token_type == TokenType::INTEGER && expression_list[0].token_value == "1"{
					if self.gen_inc_dec(target, op, target_index_expression, expression_list, function_num){
						return;
					};
				}
			}
		}

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
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("CodeGen::GEN_FUNCTION_CALL_STATEMENT:  target={}",function_name.token_value);}

		//	find the function name in the frame map
		if self.frames.frame_names.contains_key(&function_name.token_value){
			if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("CodeGen:gen_function_call_statement: {} block {}",function_name.token_value, self.frames.frame_names.get(&function_name.token_value).unwrap().0);}
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
			opcode_mode = OpcodeMode::Builtin;
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
				, 0
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
				, 0
				, Token::new()
			),function_num
		);
	}

	//	exec alloc instruction examines the specified location in the operand stack and, if there is
	//	already something there, does nothing.  If the location in the operand stack hasn't been
	//	used it creates and uninitialized variable at that location.
	fn gen_alloc(&mut self, target : &Token, detail_block_num : usize, detail_index : usize, detail_interner : usize, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){
			eprintln!("    gen_alloc: {} : {}:{}", target,detail_block_num,detail_index);
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
				, detail_interner
				, target.clone()
			),function_num
		);		
	}

	/************************************************************************
	***		Generate instructions that implement "blockable" statements   ***
	*************************************************************************/

	pub fn gen_eval(&mut self, target : &Vec<Token>, when_count : usize, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("CodeGen:GEN_EVAL: count={} target={} block_count={}", when_count, token_list_text(&target), self.block_begin_counter);}
		if self.cli.is_debug_bit(INSERT_DIAG_COMMENTS){
			self.add_machine_instruction(
				MachineInstruction::new(
					Opcode::Diag
					, OpcodeMode::NONE
					, self.symbol_table.current_frame()
					, 0
					, 0
					, Vec::new()
					, 0
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
				self.gen_alloc(&target_var, eval_data.target_detail.block_num, eval_data.target_detail.index, eval_data.target_detail.interner, function_num);
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
				, eval_data.target_detail.interner
				, target_var
			),function_num
		);

		//	add eval data to the stack
		self.eval_data.push(eval_data.clone());

		//	get the current instruction address
		let current_code_address = self.get_current_address(function_num);

		//	branch and link to the eval block
		//	current address + 1:  call the if block return to the next instruction
		//	current address + 2:  break return
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Bl
				, OpcodeMode::Bl
				, self.symbol_table.current_frame()
				, current_block_num
				, current_code_address + 1
				, vec!(0,eval_block_num, 0,0)	// not breakable
				, 0
				, Token::new()
			),function_num
		);

		//	Now all of the machine instructions will go here
		self.make_block_current(eval_block_num, function_num);
	}
	pub fn gen_when(&mut self, when_expression : &Vec<Token>, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("CodeGen::GEN_WHEN: {}", token_list_text(&when_expression));}
		if self.cli.is_debug_bit(INSERT_DIAG_COMMENTS){
			self.add_machine_instruction(
				MachineInstruction::new(
					Opcode::Diag
					, OpcodeMode::NONE
					, self.symbol_table.current_frame()
					, 0
					, 0
					, Vec::new()
					, 0
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
				, self.eval_data[eval_data_index].target_detail.interner
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
				, 0
				, Token::new()
			),function_num
		);	

		//	get the current instruction address
		let current_code_address = self.get_current_address(function_num);

		//	if the two operands were not equal. jump around
		//	the branch and link
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Jf
				, OpcodeMode::Jump
				, self.symbol_table.current_frame()
				, 0
				, current_code_address + 3   
				, Vec::new()
				, 0
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
				, current_code_address + 2
				, vec!(0, when_block_num, 0, 0)	// don't break out of when
				, 0
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
				, 0
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
						, current_code_address + 2
						, vec!(0,self.eval_data[eval_data_index].otherwise_block_num,0,0) // dont' break out of when
						, 0
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
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("CodeGen::GEN_OTHERWISE");}
		if self.cli.is_debug_bit(INSERT_DIAG_COMMENTS){
			self.add_machine_instruction(
				MachineInstruction::new(
					Opcode::Diag
					, OpcodeMode::NONE
					, self.symbol_table.current_frame()
					, 0
					, 0
					, Vec::new()
					, 0
					, Token::make_string(&format!("=== gen_otherwise ==="))
				),function_num
			);
		}

		let current_block_num = self.get_current_block_num(function_num);

		//	add a block to the block list.  save its number.  We probably can
		//	change this to current_block + 1 -- let's wait and see...
		let otherwise_block_num = self.add_code_block(false, function_num);

		//	get the current instruction address
		let current_code_address = self.get_current_address(function_num);

		//	emit the branch to the otherwise block
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Bl
				, OpcodeMode::Bl
				, self.symbol_table.current_frame()
				, current_block_num
				, current_code_address + 1
				, vec!(0,otherwise_block_num, 0, 0)	// don't break out of otherwise
				, 0
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
			eprintln!("CodeGen::GEN_IF has_else={} else_block: {} cond: {}", has_else, else_block_num, token_list_text(&condition));
		}

		let current_block_num = self.get_current_block_num(function_num);

		//	add a block to the block list.  save its number.  We probably can
		//	change this to current_block + 1 -- let's wait and see...
		let if_block_num = self.add_code_block(false, function_num);
		
		//	generate the condition evaluation code
		self.gen_expression(condition, function_num);

		//	get an address that we'll be using as a return address (with some modifications)
		//  as well as the basis for jumping around
		let current_code_address = self.get_current_address(function_num);

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
				, current_code_address + jf_offset
				, Vec::new()
				, 0
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
				, current_code_address + 2
				, vec!(0,if_block_num, 0, 0)
				, 0
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
					, current_code_address + 4					// this will be modified at block end
					, Vec::new()
					, 0
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
					, current_code_address + 3					// this will be modified at block end
					, Vec::new()
					, 0
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
					, current_code_address + 4
					, vec!(0,else_block_num, 0, 0) // don't break out of if block
					, 0
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
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("CodeGen::GEN_ELSE");}
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
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("CodeGen::GEN_WHILE");}

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
				, 0
				, Token::new()
			),function_num
		);

		// self.break_address.push ((return_block_num, post_condition_address + 3));
		// self.continue_address.push ((return_block_num, post_condition_address + 2));

		//	post_condition_address + 1 call the while loop, return to the next location
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Bl
				, OpcodeMode::Bl
				, self.symbol_table.current_frame()
				, return_block_num
				, post_condition_address + 2	// at this offset
				, vec!(1,while_block_num, return_block_num, post_condition_address + 3)
				, 0
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
				, 0
				, Token::new()
			),function_num
		);

		self.make_block_current(while_block_num, function_num);
	}
	
	//	We've added a foreach pseudo machine instruction which encodes all of the relative infomration
	//	required for the foreach header to work
	pub fn gen_foreach (&mut self, target : &Token, source_type : &TokenType, source : &Vec<Token>, function_num : usize){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("CodeGen::GEN_FOREACH target: {} source type: {} source: {}",target, source_type, source[0]);}
	
		//	get the current address in code space
		let mut current_code_address = self.get_current_address(function_num);
	
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
		//	struct member or it might be a normal variable.
		//
		//	NOTE:  we are effectively ignoring "source_type" for now.  That is, we don't support
		//	foreach on literal arrays or struct members.
		let source_detail = self.symbol_table.get_normal_address(&source[0].token_value);
		foreach_data.foreach_source_block_num = source_detail.block_num;
		foreach_data.foreach_source_address = source_detail.index;

		// foreach_data.foreach_source_detail = Some(SymbolTableEntryType::NormalSymbolEntry(source_detail));

		//	Create a target variable or use it if it alrelady exists
		match self.symbol_table.get_symbol_entry(&target.token_value){
			None		=> {
				//	add the symbol and return its detail
				foreach_data.foreach_target_detail = self.symbol_table.add_normal_symbol(&target.token_value);
				//	Generate the alloc for the new symbol

				self.gen_alloc(&target, foreach_data.foreach_target_detail.block_num, foreach_data.foreach_target_detail.index, foreach_data.foreach_target_detail.interner,  function_num);
				current_code_address += 1;
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
			Some(_) => panic!("Houston we have a problem.  gen_foreach trying to add iteration counter twice"),
			None => {} 	// this is the normal so just keep going.
		}

		//	add the symbol for the iteration index and return its detail
		foreach_data.foreach_iter_counter_detail = self.symbol_table.add_normal_symbol(&iter_counter_name);
		
		//	Generate the alloc for the new symbol
		self.gen_alloc(&foreach_data.foreach_iter_counter, foreach_data.foreach_iter_counter_detail.block_num, foreach_data.foreach_iter_counter_detail.index , foreach_data.foreach_iter_counter_detail.interner, function_num);
		current_code_address += 1;

		let iter_value_0 = Token::new2(
			TokenType::INTEGER
			, "0".to_string()
			, 0
			, "ha ha".to_string()
			, TokenCategory::Factor
		);
	
		//	Create a literal 
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Push
				, OpcodeMode::Lit
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, 0
				, iter_value_0.clone()
			),function_num
		);
		current_code_address += 1;
		
		//	Set the iteration counter (index) to 0
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Update
				, OpcodeMode::Update
				, self.symbol_table.current_frame()
				, foreach_data.foreach_iter_counter_detail.block_num
				, foreach_data.foreach_iter_counter_detail.index
				, Vec::new()
				, foreach_data.foreach_iter_counter_detail.interner
				, foreach_data.foreach_iter_counter.clone()
			),function_num
		);
		current_code_address += 1;

		//	this is where we jump when looping
		let top_of_loop = current_code_address;

		//	this is the foreach instruction
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Foreach
				, OpcodeMode::NONE
				, self.symbol_table.current_frame()
				, foreach_data.foreach_target_detail.block_num
				, foreach_data.foreach_target_detail.index
				, vec!(
						 current_code_address + 3
						,foreach_data.foreach_iter_counter_detail.block_num
						,foreach_data.foreach_iter_counter_detail.index
						,foreach_data.foreach_source_block_num
						,foreach_data.foreach_source_address
				 )
				 , foreach_data.foreach_target_detail.interner
				 , Token::new()
			),function_num
		);
		current_code_address += 1;
		//	we are in the loop now, call the foreach block and return to the
		//	the next instruction
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Bl
				, OpcodeMode::Bl
				, self.symbol_table.current_frame()
				, return_block_num
				, current_code_address + 1
				, vec!(1, foreach_block_num, return_block_num, current_code_address + 3)	// bl target and break info
				, 0
				, Token::new()
			),function_num
		);

		current_code_address += 1;

		// jump to the top of the loop
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::J
				, OpcodeMode::Jump
				, self.symbol_table.current_frame()
				, 0
				, top_of_loop
				, Vec::new()
				, 0
				, Token::new()
			),function_num
		);
		current_code_address += 1;

		//	Pop the index off the stack
		self.add_machine_instruction(
			MachineInstruction::new(
				Opcode::Pop
				, OpcodeMode::NONE
				, self.symbol_table.current_frame()
				, 0
				, 0
				, Vec::new()
				, 0
				, Token::new()
			),function_num
		);

		//  And remove it from the symbol table
		self.symbol_table.remove_symbol(&iter_counter_name);

		self.break_address.push((return_block_num, current_code_address+1));
		
		//	start adding instructions to the new foreach block
		self.make_block_current(foreach_block_num, function_num);
	}

	//	This doesn't actually generate any code.  Instead, it adds the literal symbol and value
	//	to the symbol table so that when an expression references that symbol, it'll find the
	//	value and a Push-Lit can happen
	pub fn gen_literal_statement(&mut self, literal_id : &Token, literal_value : &Vec<Token>){
		if self.cli.is_debug_bit(TRACE_CODE_GEN){eprintln!("CodeGen::GEN_LITERAL {} = {}",literal_id.token_value, token_list_text(literal_value));}

		let value_token = literal_value[0].clone();

		match value_token.token_type{
			TokenType::FLOAT | TokenType::INTEGER => {
				self.symbol_table.add_literal(&literal_id.token_value, &LiteralType::LiteralNumber(LiteralNumber::new(value_token.token_value.parse::<f64>().unwrap())));
			}

			TokenType::STRING => {
				self.symbol_table.add_literal(&literal_id.token_value, &LiteralType::LiteralString(LiteralString::new(&value_token.token_value)));
			}

			TokenType::BOOL => {
				if value_token.token_value == "true"{
					self.symbol_table.add_literal(&literal_id.token_value, &LiteralType::LiteralBool(LiteralBool::new(true)));
				}else if value_token.token_value == "false"{
					self.symbol_table.add_literal(&literal_id.token_value, &LiteralType::LiteralBool(LiteralBool::new(false)));
				}
			}

			_=> panic!("from gen_literal_statement: {} not supported", value_token),
		}
	}
}