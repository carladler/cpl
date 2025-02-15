
#![allow(unused_imports)]

use tokenizer::*;
use cli::*;
use cplvar::*;
use builtin::*;
use opcode::*;
use machineinstruction::*;
use codeframe::*;
use macrolib::*;
use runtimestats::*;
use std::time::SystemTime;
use std::time::{Duration, Instant};
use std::collections::HashMap;


//	These enums indicate the data types of two operands
enum OperandAnalysis{
	InvalidType,
	StringString,
	StringNumber,
	StringBool,
	StringArray,
	NumberString,
	NumberNumber,
	NumberBool,
	NumberArray,
	BoolString,
	BoolNumber,
	BoolBool,
	BoolArray,
	ArrayString,
	ArrayNumber,
	ArrayBool,
	ArrayArray,
}

//	These are the types that are possible (any other types discovered
//	are illegal)
#[derive(PartialOrd, Ord, PartialEq, Copy, Clone, Eq, Hash)]
enum OperandType{
	OtString,
	OtNumber,
	OtBool,
	OtArray,
}



//	An Executor contains all of the code, operands and processing data
//	for the execution of a single function.  There is no communication
//	between functions except for call (arguments) and return (return_value)
pub struct Executor<'a>{
	cli : & 'a CLI<'a>,

	//  A refernce to the vector containing the list of code frames from
	//	FrameMap.  We need them all here so that function calls can find them
	//	CodeFrame = Function
	code_frames : & 'a Vec<CodeFrame>,

	//	This is the code frame we're currently executing
	//	NB:  This numer includes all of the built-in functions;
	//	the initial number is built-in function count + 1
	code_frame_num : usize,

	//	This is the current code block of the current frame
	code_block_num : usize,

	//	This is a return address. It contains both the block number and offset.  It is
	//	where the BlockEnd instruction jumps to
	block_end_return_info : Vec<(bool, usize, usize)>,

	//	This is the address to transfer control when breaking.  Entries with
	//	.0 = false are ignored (except that their variables are removed)
	break_info : Vec<(bool, usize, usize)>,

	//	All the currently active variables and the frame numbers
	//	An operand frame is created whenever the executor is
	//	created (i.e. whenever a function is called) and is
	//	destoryed when control returns to a callling function
	operand_stack : & 'a mut OperandStack,


	//	The arguments that were passed to this function by a calling
	//	function
	arguments : & 'a mut Vec<CplVar>,		// arguments passed to this function

	//	A reference to the builtin function table which was
	//	built as part of code generation.  It never changes during
	//	execution
	builtin_functions : & 'a mut BuiltinFunctions,

	//	the number of arguments passed (maybe?)
	arg_count : usize,

	// process variables
	//	the current instruction being executed
	instruction_counter : usize,

	//	The value returned by a called function
	return_value : CplVar,					// returns the value of the return opcodes

	//	just keep track of begin and ends for sanity checking
	block_counter : i32,

	//	hmmmm, just a diagnostic
	_call_flag : bool,

	runtime_data : & 'a mut RuntimeData,
	runtime_data_qual : String,
	names : & 'a Vec<String>,
}

impl<'a> Executor<'a>{
	pub fn new(
				cli : & 'a CLI<'a>
				, frame_map : & 'a mut FrameMap
				, arguments : & 'a mut Vec<CplVar>
				, operand_stack : & 'a mut OperandStack
				, arg_count : usize
				, runtime_data : & 'a mut RuntimeData
				, names : & 'a Vec<String>,
	    	) -> Executor<'a> {
		Executor{
			cli : cli,
			code_frames : &frame_map.frames_list,
			code_frame_num : frame_map.get_entry_frame_number(),
			code_block_num : 0,				// always start at the first code block
			block_end_return_info : Vec::new(),
			break_info : Vec::new(),
			operand_stack : operand_stack,
			builtin_functions : &mut frame_map.builtin_function_table,
			arguments : arguments,

			arg_count : arg_count,
			instruction_counter : 0,
			return_value : CplVar::new(CplDataType::CplUninitialized(CplUninitialized::new())),
			block_counter : 0,
			_call_flag : false,
			runtime_data : runtime_data,
			runtime_data_qual : String::new(),
			names : names,
		}
	}


	pub fn call(
		cli : & 'a CLI<'a>
		, code_frames : & 'a Vec<CodeFrame>
		, code_frame_num : usize
		, arguments : & 'a mut Vec<CplVar>
		, operand_stack : & 'a mut OperandStack
		, builtin_functions : & 'a mut BuiltinFunctions
		, arg_count : usize
		, runtime_data : & 'a mut RuntimeData
		, names : & 'a Vec<String>,
		) -> Executor<'a> {
	
		Executor{
			cli : cli,
			code_frames : code_frames,
			code_frame_num : code_frame_num,
			code_block_num : 0,				// always start at the first code block
			block_end_return_info : Vec::new(),
			break_info : Vec::new(),
			operand_stack : operand_stack,
			builtin_functions : builtin_functions,
			arguments : arguments,

			arg_count : arg_count,
			instruction_counter : 0,
			return_value : CplVar::new(CplDataType::CplUninitialized(CplUninitialized::new())),
			block_counter : 0,
			_call_flag : true,
			runtime_data : runtime_data,
			runtime_data_qual : String::new(),
			names : names,
		}
	}

	pub fn get_runtime_data(&self) -> HashMap<InstructionKey,EventPayload>{
		return self.runtime_data.accumulated_events.clone();
	}

	//	When we know we need a number from the operand stack, get it and return it
	//  here.
	fn pop_to_number (&mut self, instruction : &MachineInstruction) -> f64{
		let tos= self.operand_stack.pop();
		match tos.var{
			CplDataType::CplNumber(n) => n.cpl_number,
			CplDataType::CplString(ref s) => {
				match s.cpl_string.parse::<f64>(){
					Ok(n) => n as f64,
					Err(_) => panic!("ERROR: operation \"{}\" can only be performed on numbers! Operand is \"{}\"", instruction.opcode, s.cpl_string),
				}
			},
			_ => {
				panic! ("Operation {} can't be performed on {}!", instruction.opcode,tos.var);
			},
		}
	}

	
	fn dump_operands(&self, title : &str){
		eprintln!("{}", title);
		self.operand_stack.dump_operands();
	}

	pub fn exit(&mut self, exit_code : &CplVar){
		match exit_code.var{
			CplDataType::CplNumber(ref n) => if n.cpl_number as i32 != 0{
				eprintln!("Program exiting with: {}", n.cpl_number);
				std::process::exit(n.cpl_number as i32); 
			},

			CplDataType::CplString(ref s) => {
				if s.cpl_string != "$$Synthetic$$"{
					eprintln!("Program exit with: {}", s.cpl_string);
				}
			},

			CplDataType::CplUninitialized(_)=> {},
			
			_ => eprintln!("Program exit with: {}", exit_code),
		}
	}


	//	This is the "interpreter" of the opcodes.  Each instruction is read from the frame and
	//	the associated function in this object is called.  Each of these functions calls it's
	//	sister function in the OperandStack object which is "closer" to the data these functions
	//	operate on.

	//	When control is passed to this executor, the operand stack is empty and the instruction
	//	counter is set 0.  After each instruction is read, the instrction counter is incremented
	//	so that if a function call is executed, we know that the return address is the instruction
	//	counter value of the call + 1.

	//	When exec completes, it returns the value of the Return expression (if any)

	pub fn exec (&mut self) -> CplVar{
		//	At the start of execution, create a new operand stack frame for variables associated with
		//	this executor.  With exception of variables passed by reference, access to a variable is
		//	VIA the last frame on operand stack.
		self.operand_stack.push_frame();

		if self.cli.is_debug_bit(DUMP_OPERANDS){self.dump_operands("At fn exec");}
		// if cfg!(feature="debugtrace"){
		// 	eprintln!("blah blah");
		// 	if self.cli.is_debug_bit(DUMP_OPERANDS){self.dump_operands("At fn exec");}
		// }else{
		// 	eprintln!("bleet bleet");
		// }

		while self.instruction_counter < self.code_frames[self.code_frame_num].code_block_list[self.code_block_num].code_block.len() {
			//	get the current instruction
			let instruction = self.code_frames[self.code_frame_num].code_block_list[self.code_block_num].code_block.get(self.instruction_counter).unwrap();
			if self.cli.is_debug_bit(TRACE_EXEC_DISPATCHER){eprintln!("exec(dispatcher): {}:{} {}",self.code_block_num, self.instruction_counter, instruction);}

			self.runtime_data.mark_begin();

			match instruction.opcode{

				Opcode::BlockBegin 				=> self.exec_block_begin(instruction),
				Opcode::BlockEnd				=> self.exec_block_end(instruction),
				Opcode::Push 					=> self.exec_push(instruction),
				Opcode::PushNewCollection		=> self.exec_push_new_collection(instruction),

				Opcode::Print 					=> self.exec_print(instruction.clone(),true, false),
				Opcode::Eprint					=> self.exec_print(instruction.clone(),false, false),
				Opcode::Println					=> self.exec_print(instruction.clone(),true, true),
				Opcode::Eprintln				=> self.exec_print(instruction.clone(),false, true),

				Opcode::Pop 					=> self.exec_pop(instruction),

				Opcode::Add 					=> self.exec_binary_operator(instruction),
				Opcode::Sub 					=> self.exec_binary_operator(instruction),
				Opcode::Mul 					=> self.exec_binary_operator(instruction),
				Opcode::Div 					=> self.exec_binary_operator(instruction),
				Opcode::Mod 					=> self.exec_binary_operator(instruction),
				Opcode::BwOr 					=> self.exec_binary_operator(instruction),
				Opcode::BwAnd 					=> self.exec_binary_operator(instruction),
				Opcode::Concat 					=> self.exec_binary_operator(instruction),


				Opcode::AddEq 					=> self.exec_update(instruction),
				Opcode::SubEq 					=> self.exec_update(instruction),
				Opcode::MulEq 					=> self.exec_update(instruction),
				Opcode::DivEq					=> self.exec_update(instruction),
				Opcode::ModEq 					=> self.exec_update(instruction),
				Opcode::OrEq 					=> self.exec_update(instruction),
				Opcode::AndEq 					=> self.exec_update(instruction),
				Opcode::AppendEq 				=> self.exec_update(instruction),
				Opcode::Update					=> self.exec_update(instruction),

				Opcode::Append					=> self.exec_append(instruction),
				Opcode::Insert					=> self.exec_insert(instruction),

				Opcode::Lor 					=> self.exec_lor_land(instruction),
				Opcode::Land 					=> self.exec_lor_land(instruction),

				Opcode::Inc 					=> self.exec_inc_dec(instruction),
				Opcode::Dec 					=> self.exec_inc_dec(instruction),
				Opcode::Uminus 					=> self.exec_unary_op(instruction),	
				Opcode::Damnit					=> self.exec_unary_op(instruction),
				Opcode::LengthOf				=> self.exec_length_of(),

				Opcode::J						=> self.exec_j(instruction),
				Opcode::Jt						=> self.exec_jt(instruction),
				Opcode::Jf						=> self.exec_jf(instruction),
				Opcode::Bl						=> self.exec_bl(instruction),

				Opcode::Break					=> self.exec_break(instruction),
				Opcode::Continue				=> self.exec_continue(instruction),

				Opcode::Lt						=> self.exec_binary_operator(instruction),
				Opcode::Gt						=> self.exec_binary_operator(instruction),
				Opcode::Le						=> self.exec_binary_operator(instruction),
				Opcode::Ge						=> self.exec_binary_operator(instruction),
				Opcode::Ne						=> self.exec_binary_operator(instruction),
				Opcode::Eq						=> self.exec_binary_operator(instruction),
	
				Opcode::FunctionCall			=> self.exec_function_call(instruction),
				Opcode::FetchIndexed			=> self.exec_fetch_indexed(instruction),

				Opcode::IncArgCount 			=> self.arg_count += 1,
				Opcode::Return					=> self.exec_return(instruction),
				Opcode::Exit					=> self.exec_exit(instruction),

				Opcode::Alloc					=> self.exec_alloc(instruction),

				Opcode::Diag					=> self.exec_diag(instruction),

				Opcode::Foreach					=> self.exec_foreach(instruction),

				_ => abend!(format!("{} Not Implemented Yet", instruction.opcode)),
			}

			if self.cli.is_runtime_stats_enabled(){
				self.runtime_data.mark_end(instruction, self.runtime_data_qual.clone());
				self.runtime_data_qual.clear();
			}

			if self.cli.is_debug_bit(DUMP_OPERANDS_DISPATCH){
				eprintln! ("instruction: {}", instruction);
				self.dump_operands("After instruction Execution");
			}

			//	Don't increment if the instruction counter was set by the following
			//	because those opcodes set the next address to get the instruction from rather than
			//	the next opcode in the instruction stream.  Return is kind of a special case because
			//	if we're in the entry function, we break out of the execution loop.
			match instruction.opcode{
				Opcode::Foreach |
				Opcode::Bl |
				Opcode::J  |
				Opcode::Jt |
				Opcode::Jf |
				Opcode::BlockEnd |
				Opcode::Break |
				Opcode::Continue		=> {}
				
				//	After executing the return instruction which adds the return
				//	value to return value register we break out of the exec loop
				//	because we're done with the function.  Recall that an active
				//	function is an instantiation of the executor object
				Opcode::Return			=> break,

				Opcode::Exit			=> break,

				_						=> self.instruction_counter += 1,
			}
		}


		if self.cli.is_debug_bit(TRACE_EXEC_DISPATCHER){eprintln!("exec(dispatcher -- exit): {}:{}",self.code_block_num, self.instruction_counter);}
		self.operand_stack.pop_frame();
		return CplVar::new(self.return_value.var.clone());
	}

	//	create a new operand block (e.g. for a WHILE instruction) in the current frame
	//	I can't think of reason to support creating a block in another frame
	fn exec_block_begin(&mut self, instruction : &MachineInstruction){		
		if self.cli.is_debug_bit(TRACE_EXEC){
			eprintln!("{}:{} : exec_block_begin: {} block {}", self.code_block_num, self.instruction_counter, instruction, self.block_counter);
		}

		self.block_counter += 1;
		self.operand_stack.push_block();

		//self.dump_operands("At exec_block_begin");
		if self.cli.is_debug_bit(DUMP_OPERANDS){self.dump_operands("At exec_block_begin");}
	}

	//	End a block:  get the return address from a stack
	//	pop the operand stack for the block
	//	transfer control to the return address
	//
	//	If there isn't any return addresses on the stack
	//	then this is the end of the function and we just
	//	increment the instruction counter and be done
	fn exec_block_end(&mut self, instruction : &MachineInstruction){

		if self.block_end_return_info.is_empty(){
			if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_block_end: {} block_len {}", self.code_block_num,self.instruction_counter, instruction, self.operand_stack.variable_count())}
			self.instruction_counter += 1;
			return;
		}

		//	get the return block_num, address
		let block_end_return_info = self.block_end_return_info.pop().unwrap();

		//	and get rid of the break_info
		self.break_info.pop();

		if self.cli.is_debug_bit(TRACE_EXEC){
			eprintln!("{}:{} : exec_block_end: {} returning to: {}:{}", 
				self.code_block_num, self.instruction_counter, self.code_block_num, block_end_return_info.1, block_end_return_info.2);
		}

		self.code_block_num = block_end_return_info.1;
		self.instruction_counter = block_end_return_info.2;


		self.block_counter -= 1;


		//	pop the operand stack frame
		self.operand_stack.pop_block();

	}

	//	Fetch a value from an array or dictionary using indices.  The stack is expected
	//	to look like this:
	//
	//		array (var_ref)
	//		index 0
	//		index 1
	//			:
	//		index n
	//
	//	where the CPL expression is:  array[index0, index1, ... indexn];
	//
	fn exec_fetch_indexed(&mut self, instruction: &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_fetch_indexed: {}", self.code_block_num,self.instruction_counter, instruction)}

		//	if the number of indices is not mentioned or, if mentioned is 1 then just do a normal
		//	fetch_indexed
		let index_count : usize;
		if instruction.qualifier.len() == 0 || instruction.qualifier[0] == 0{
			index_count = 1;
		}else{
			index_count = instruction.qualifier[0];
		}

		let tos_ix = self.operand_stack.operand_frames.last().unwrap().operand_blocks.last().unwrap().operand_block.len() - 1;

		if let CplDataType::CplVarRef(ref varref) = self.operand_stack.operand_frames.last().unwrap().operand_blocks.last().unwrap().operand_block.get(tos_ix-index_count).unwrap().var{
			if let CplDataType::CplArray(_) = self.operand_stack.operand_frames.get(varref.frame_num).unwrap().operand_blocks.get(varref.block_num).unwrap().operand_block.get(varref.address).unwrap().var{
				let fetched = self.operand_stack.fetch_array_indexed_from_operand_stack(index_count);		
				self.operand_stack.push(&fetched);
			}else if let CplDataType::CplDict(_) = self.operand_stack.operand_frames.get(varref.frame_num).unwrap().operand_blocks.get(varref.block_num).unwrap().operand_block.get(varref.address).unwrap().var{
				let fetched = self.operand_stack.fetch_dict_indexed_from_operand_stack(index_count);		
				self.operand_stack.push(&fetched);
			}else{
				panic!("from exec_fetch_indexed: Can only index an array or dictionary.  Got {}", self.operand_stack.operand_frames.get(varref.frame_num).unwrap().operand_blocks.get(varref.block_num).unwrap().operand_block.get(varref.address).unwrap().var);
			}
		}else{
			self.dump_operands("================ at exec_fetch_indexed");
			panic!("from exec_fetch_indexed: Can only index via a VarRef.  Got {}", self.operand_stack.operand_frames.last().unwrap().operand_blocks.last().unwrap().operand_block.get(tos_ix-index_count).unwrap());
		}

	}

	//	Push instructions always operate on the current top of the operand stack (i.e.
	//	the current operand frame)

	fn push_lit_string(&mut self, instruction: &MachineInstruction, _instruction_address : usize){
		//	get the current operand_frame
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("     push_lit_string: {}", instruction)}

		self.operand_stack.push(&CplVar::new(
			CplDataType::CplString(CplString::new(instruction.literal.token_value.clone()))));
	}

	fn push_lit_number(&mut self, instruction: &MachineInstruction, _instruction_address : usize){
		//	get the current operand_frame
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("      push_lit_number: {}", instruction)}

		let mut rust_type : RustDataType = RustDataType::NONE;

		match instruction.literal.token_type{
			TokenType::INTEGER => rust_type = RustDataType::Int,
			TokenType::FLOAT => rust_type = RustDataType::Real,
			TokenType::STRING => rust_type = RustDataType::String,
			TokenType::BOOL => rust_type = RustDataType::Bool,
			_ => abend!(format!("token_type_to_rust_type:  unknown token_type: {}",rust_type)),
		}

		self.operand_stack.push(&CplVar::new(
			CplDataType::CplNumber (CplNumber::new (rust_type, instruction.literal.token_value.parse::<f64>().unwrap()))));
	}

	//	used by push_lit_bool and comparison ops
	fn push_lit_bool_help(&mut self, b : bool, instruction_address : usize, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : push_lit_bool_help: {}", self.code_block_num,instruction_address, instruction)}
		self.operand_stack.push(&CplVar::new(CplDataType::CplBool (CplBool::new (b))));
	}

	//	used by exec_push
	fn push_lit_bool(&mut self, instruction: &MachineInstruction, instruction_address : usize){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : push_lit_bool: {}", self.code_block_num, instruction_address, instruction)}
		let b = instruction.literal.token_value.parse::<bool>().unwrap();
		self.push_lit_bool_help(b, instruction_address, instruction);
	}

	//	used by exec_push to create an uninitialized variable
	fn push_lit_none(&mut self, instruction: &MachineInstruction){
		//	get the current operand_frame
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("     push_lit_none: {}", instruction)}

		self.operand_stack.push(&CplVar::new(CplDataType::CplUninitialized(CplUninitialized::new())));
	}

	fn push_lit(&mut self, instruction : &MachineInstruction, instruction_address : usize){
		if instruction.literal.token_type == TokenType::INTEGER || instruction.literal.token_type == TokenType::FLOAT 	{
			self.push_lit_number(instruction, instruction_address);
		}else if instruction.literal.token_type == TokenType::STRING{
			self.push_lit_string(instruction, instruction_address);
		}else if instruction.literal.token_type == TokenType::BOOL{
			self.push_lit_bool(instruction, instruction_address);
		}else if instruction.literal.token_type == TokenType::NONE{
			self.push_lit_none(instruction);
		}else{
			eprintln!("push_lit: {}", instruction.literal.token_type);
		}
	}

	//	If the operand is a scalar (e.g. number) or a VarRef then just push it onto the
	//	stack.  If the operand is a collection, create a VarRef pointing at it
	//	and push that onto the stack.
	fn push_copy(&mut self, operand : &CplVar, instruction : &MachineInstruction){
		// if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("      push_copy: operand={}, instruction={}",operand, instruction)}

		match operand.var{
			CplDataType::CplNumber(_)			|
			CplDataType::CplString(_)			|
			CplDataType::CplBool(_) 			|
			CplDataType::CplUninitialized(_)	|
			CplDataType::CplUndefined(_) 		=> {
				self.runtime_data_qual = "Scalar".to_string();
				if instruction.opcode_mode == OpcodeMode::VarRef{
					if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("      push_copy(VarRef) {},{},{}",  self.operand_stack.current_frame(), instruction.block_num, instruction.address)};
					self.operand_stack.push(&CplVar::new(CplDataType::CplVarRef(CplVarRef::new(self.operand_stack.current_frame(), instruction.block_num, instruction.address))));
				}else{
					if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("      push_copy(mode={}) {}", instruction.opcode_mode, instruction)};
					self.operand_stack.push(operand);
				}
			}
			
			CplDataType::CplVarRef(_)					=> {
				self.runtime_data_qual = "VarRef".to_string();
				if instruction.opcode_mode == OpcodeMode::VarRef{
					if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("      push_copy(VarRef) {},{},{}",  self.operand_stack.current_frame(), instruction.block_num, instruction.address)};
					self.operand_stack.push(&CplVar::new(CplDataType::CplVarRef(CplVarRef::new(self.operand_stack.current_frame(), instruction.block_num, instruction.address))));
				}else{
					if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("      push_copy(mode={}) {}", instruction.opcode_mode, instruction)};
					self.operand_stack.push(operand);
				}
			}

			CplDataType::CplArray(_) | CplDataType::CplDict(_) =>{
				self.runtime_data_qual = "Collection".to_string();
				if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("      push_copy(Collection) {} {}", operand, instruction)};
				self.operand_stack.push(&CplVar::new(CplDataType::CplVarRef(CplVarRef::new(self.operand_stack.current_frame(), instruction.block_num, instruction.address))));
			},

			_ => abend!(format!("From push_copy: I don't know what {} means", operand.var)),
		}
	}

	//	Arguments are passed to a function via the arguments array in the exec object.
	//	(We thin of this as the "arguments register").
	//
	//  This opcode grabs the argument at the top of the arguments register and pushes it
	//	onto the operand stack.  Theoretically, these vars will map directly onto the parameter
	//	addresses that were in the symbol table. In other words, the parameters are really
	//	aliases to the arguments.
	fn push_arg(&mut self, instruction : &MachineInstruction, _instruction_address : usize){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("      push_arg: {}", instruction)}

		//	fetch the next argument from the arguments register
		let mut arg = self.arguments.pop().unwrap();
		arg.interner = instruction.interner;

		//	and push it onto the stack
		self.operand_stack.push(&arg);
	}

	//	Push Array means:  create a new empty array at the top of the stack
	// fn push_array(&mut self, instruction : &MachineInstruction, _instruction_address : usize){
	// 	if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("     push_array: {}",instruction)}
	// 	self.operand_stack.push(&CplVar::new(CplDataType::CplArray(CplArray::new())));
	// }

	// //	Push Dict means:  create a new empty dictionary rray at the top of the stack
	// fn push_dict(&mut self, instruction : &MachineInstruction, instruction_address : usize){
	// 	if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("     push_dict: {}", self.code_block_num, instruction_address, instruction)}
	// 	self.operand_stack.push(&CplVar::new(CplDataType::CplDict(CplDict::new())));
	// }


	//	Create a new collection on the opernand stack.  The mode says what kind of
	//	collection.  This will be the only actual instance of the collection.  All other
	//	"instances" will be VarRef's to it (I hope).
	fn exec_push_new_collection(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_push_new_collection: {} Mode: {}", self.code_block_num, self.instruction_counter,instruction, instruction.opcode_mode)}
		match instruction.opcode_mode{
			OpcodeMode::Array		=> {
				self.operand_stack.push(&CplVar::inew(CplDataType::CplArray(CplArray::new()),instruction.interner));
				//self.push_array(instruction, self.instruction_counter)
			}
			OpcodeMode::Dict		=> {
				self.operand_stack.push(&CplVar::new(CplDataType::CplDict(CplDict::new())));
				// self.push_dict(instruction, self.instruction_counter)
			},

			_=> abend!(format!("From exec_push: I don't know this address mode {}", instruction.opcode_mode)),
		}
	}

	//	The instruction is to push something onto the operand stack. The "something" is:
	//
	//		* a literal value (from the instruction)
	//		* a variable whose "local" address is in the instruction
	//		* an argument from from the arguments register
	//		* an array element -- technically this is not going onto the stack but
	//		  instead it's being added to an array which is at the top of the stack
	//		  (this is used for struct instantiation -- see exec-update (mode = array)
	//		  which does something similar -- maybe even the same thing)
	fn exec_push(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_push: {} Mode: {}", self.code_block_num, self.instruction_counter,instruction, instruction.opcode_mode)}
		match instruction.opcode_mode{
			OpcodeMode::Lit 			=> self.push_lit(instruction, instruction.address),
			OpcodeMode::Var =>{
				//	we need to make a copy of the var indicated by instruction' block and address
				let operand = self.operand_stack.fetch_local_var(instruction.block_num, instruction.address);
				self.push_copy(&operand, instruction);	
			}
			OpcodeMode::VarRef=>{
				//	we need to create a VarRef on the operand stack pointing to an operand
				let operand = self.operand_stack.fetch_local_var(instruction.block_num, instruction.address);
				if let CplDataType::CplVarRef(_) = operand.var{
					self.operand_stack.push(&operand);
				}else{
					self.push_copy(&operand, instruction);
				}
			}
			OpcodeMode::Arg				=> self.push_arg(instruction, self.instruction_counter),

			//	If we're sitting on top of an array, we can just add the element directly to it
			OpcodeMode::Array			=> self.operand_stack.push_array_element(instruction.interner),

			_=> abend!(format!("From exec_push: I don't know this address mode {}", instruction.opcode_mode)),
		}

		if self.cli.is_debug_bit(DUMP_OPERANDS){self.dump_operands("after exec_push");}
	}

	//	Removes the item at the top of the operand stack
	fn exec_pop(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_pop: {}", self.code_block_num, self.instruction_counter, instruction)}
		
		//	pop the stack but throw it away
		self.operand_stack.pop();
	}

	

	//	println always uses whatever is at the top of the current frame.  If that var
	//	is  VarRef then println whatever it's pointing at.  NOTE:  we only support
	//	a single level of indirection.  If the thing that the VarRef is pointing at
	//	is, itself, a VarRef, the println instruction fails.

	//	the is_print parameter controls whether print(ln) or eprint(ln) is used (true -> print, false -> eprint)
	//	the has_nl parameter controls whether or not a new new line is appened to the ouptut (true -> append new line)
	fn exec_print(&mut self, instruction : MachineInstruction, is_print : bool, has_nl : bool){

		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_print: {}", self.code_block_num, self.instruction_counter, instruction)}

		let mut tos_ref = self.operand_stack.fetch_tos_ref();
		if let CplDataType::CplVarRef(ref vr) = tos_ref.var {
			tos_ref = self.operand_stack.fetch_ref(vr.frame_num, vr.block_num, vr.address);
		}

		#[macro_export]
		macro_rules! exec_print{
			($var:expr, $is_print:expr, $has_nl:expr) =>{
				{
					if $is_print && $has_nl{
						print!("{}\n",$var);
					}else if $is_print && !$has_nl{
						print!("{}\n",$var);
					}else if !$is_print && $has_nl{
						eprint!("{}\n",$var);
					}else if !$is_print && !$has_nl{
						eprint!("{}",$var);
					}
				}
			}
		}

		match tos_ref.var{
			CplDataType::CplNumber(ref v) => {
				exec_print!(v.cpl_number, is_print, has_nl);
			}

			CplDataType::CplString(ref v) => {
				exec_print!(v.cpl_string, is_print, has_nl);
			}

			CplDataType::CplBool(ref v) => {
				exec_print!(v.cpl_bool, is_print, has_nl);
			}

			CplDataType::CplArray(ref v) => {
				exec_print!(v, is_print, has_nl);
			}
			_ => eprintln!("Can't print: {}", tos_ref.var),
		}


		//	When we're done printing, consume the top of stack
		self.operand_stack.pop();
	}

	//  Apply an operator to a scalar lvalue.  The value may be either direct or via
	//	a VarRef (if this is a called function with a pass by reference argument)
	fn apply_assignment_operator(&mut self, block_num : usize, address : usize, opcode : Opcode) {
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("     assignment_operator: {}", opcode);}

		//	determine if the TOS is the variable to update or is a VarRef pointing at it.  Note that
		//	we are only support a single level of indirection (i.e. we can't deal with a VarRef pointint at another
		//	VarRef)
		match self.operand_stack.fetch_local_var(block_num, address).var{
			CplDataType::CplNumber(_) | CplDataType::CplString(_) => {
				//	If the collection is direct (i.e. an array or dictionay) do this
				// n.apply_binary_operator_to_number(&new_value_var, instruction.opcode);
				self.operand_stack.apply_binary_operator_scalar_local(block_num, address, opcode);
			}

			CplDataType::CplVarRef(vr) =>{
				//	Get the address of the lvalue via the VarRef
				// let varref : &mut CplVar = self.operand_stack.fetch_mutable_ref(vr.frame_num, vr.block_num, vr.address);
				self.operand_stack.apply_binary_operator_scalar_global(vr.frame_num, vr.block_num, vr.address, opcode);
			}

			_=> panic!("from exec.apply_assignment_operator:  unknown type: {}", self.operand_stack.fetch_local_var(block_num, address).var),
		}
	}



	//	This updates a local operand in situ via the address in the instruction.  Whether this
	//	is a simple replacement or an operator assignment is determined by the opcode
	fn update_scalar_tos(&mut self, block_num : usize, address : usize, op : Opcode){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("      update_scalar_tos(start): {},{} op={}", block_num, address, op)}

		match op{
			Opcode::Update => self.operand_stack.update_local_from_tos(block_num, address),
			_ => self.apply_assignment_operator(block_num, address, op),
		}
		//if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("      update_scalar_tos: {},{} {}", block_num, address, self.operand_stack.fetch_local_var(block_num, address))}
	}

	//	If the mode is UpdateIndexed, the indicies are on the stack and the number of
	//	indicies is the first element of the qualifier.
	//
	//	If the mode is UpdateStructMember, the indices are in the qualfifier.  Structs
	//	are always arrays so the indicies are always numbers.
	fn update_indexed_direct(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){
			eprintln!("      update_indexed_direct: {} {} index_list={:?} op={}", instruction, self.operand_stack.fetch_local_var(instruction.block_num, instruction.address), instruction.qualifier, instruction.opcode);
			//self.dump_operands("========== at update_indexed_direct");
		}

		self.operand_stack.update_local_collection(instruction.block_num, instruction.address, &instruction.qualifier, instruction.opcode, instruction.opcode_mode);
		self.runtime_data_qual = "Indexed".to_string();
	}

	//	As noted, this is a bit tricky:  we need to get a rust reference to the
	//	operand pointed to by the VarRef
	fn update_indexed_indirect(&mut self, instruction : &MachineInstruction, current_frame_num : usize){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("      update_indexed_indirect: {} {}", instruction, self.operand_stack.fetch_local_var(instruction.block_num, instruction.address))}
	
		let collection_ref = &self.operand_stack.operand_frames[current_frame_num].operand_blocks[instruction.block_num].operand_block[instruction.address];
		if let CplDataType::CplVarRef(ref vr) = &collection_ref.var{
			//	So here is an example of getting around the borrow checker.  You'd like to
			//	think that you could use the fields of vr (var ref) directly in the next
			//	statement below ("let collection = {...}").  But NO!.  If you substitute
			//	vr.frame_num for coll_frame in that statement you get:
			//
			//	cannot borrow `self.operand_stack.operand_frames` as mutable because it is also borrowed as immutable
			//
			//	so, the solution, evidently, is to create local variables to hold these fields.
			let frame_num = vr.frame_num;
			let block_num = vr.block_num;
			let address = vr.address;

			if self.cli.is_debug_bit(TRACE_EXEC){
				eprintln!("      update_indexed_indirect: frame={} block={} address={}", frame_num, block_num, address);
			}

			self.operand_stack.update_global_collection(frame_num, block_num, address, &instruction.qualifier, instruction.opcode, instruction.opcode_mode);
			self.runtime_data_qual = "Indexed-Indirect".to_string();
		}
	}


	//	Here we are updating an element of an array or dictionary.  The Address
	//	of the collection is in the instruction.  This will be the case if the collection
	//	is declared in the same function as where it was defined.  If the address points at a VarRef
	//	the VarRef points at the collection.  This will happen when the collection was passed as
	//	an argument to a function:  collections are always passed by reference.
	//
	fn update_indexed (&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("      update_indexed: {} {} indices={:?}", instruction, self.operand_stack.fetch_local_var(instruction.block_num, instruction.address),instruction.qualifier)}

		let frame_num = self.operand_stack.operand_frames.len()-1;

		match self.operand_stack.operand_frames[frame_num].operand_blocks[instruction.block_num].operand_block[instruction.address].var{
			CplDataType::CplVarRef(_) 	=> self.update_indexed_indirect(&instruction, frame_num),
			_ 							=> self.update_indexed_direct(&instruction),
		}
	}

	//	Adds a Var to an array that is at the top of the stack.  Tos is the value to update.
	//	Tos-1 is the array to update.
	fn update_collection_array(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("      update_collection_array: {} {} (interner: {})", self.code_block_num, self.instruction_counter, instruction.interner)}
		self.operand_stack.push_array_element(instruction.interner);
	}

	//	adds a new key/value pair to a dictionary.  The key/value pair are at tos and tos-1.
	//	The dictionary is at tos-2
	fn update_collection_dict(&mut self){
		self.operand_stack.insert_dict();
	}

	//	Update an Lvalue
	//	If the mode is Update, then update the scalar value at the address specified in the instruction
	//  If the mode is UpdateIndexed, see comments at "update_indexed"
	fn exec_update(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_update: {}", self.code_block_num, self.instruction_counter, instruction)}

		//self.dump_operands("begin exec_update");

		//	Update:					update a scalar
		//	UpdateStructMember:		update the struct array (indices in instruction.qualifier)
		//	UpdateIndexed:			update an normal array (Indices on stack, count in instruction.qualifier)
		match instruction.opcode_mode{
			OpcodeMode::Update => {
				self.update_scalar_tos(instruction.block_num, instruction.address, instruction.opcode);
				self.runtime_data_qual = "Scalar".to_string();
			}

			OpcodeMode::UpdateStructElement | OpcodeMode::UpdateIndexed => {
				self.update_indexed(instruction);
				self.runtime_data_qual = "Indexed".to_string();
			}
			_=> abend!(format!("From exec_update:  I don't know what this means: {}", instruction.opcode_mode)),
		}

		if self.cli.is_debug_bit(DUMP_OPERANDS){self.dump_operands("at exec_update")}
	}

	//	Appends the array at tos with the value at tos-1
	fn exec_append(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_append: {}", self.code_block_num, self.instruction_counter, instruction)}
		self.update_collection_array(instruction);
	}

	//	Inserts a key/value pair into a dictionary at tos from the key at tos-1 and the value at tos-2
	//	Appends the array at tos with the value at tos-1
	fn exec_insert(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_insert: {}", self.code_block_num, self.instruction_counter, instruction)}
		self.update_collection_dict();
	}


	
	//	Apply and operator to an element of a collection
	fn _apply_binary_operator_indexed(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("     apply_binary_operator_indexed {}", instruction)}

		let var = self.operand_stack.fetch_local_var(instruction.block_num, instruction.address);
	
		//	Don't dereference the new value as it might be a reference to an array
		let new_value = self.operand_stack.pop();

		//	However, the index always is a scalar
		let index = self.operand_stack.dereference_tos();
	
		match var.var{

			CplDataType::CplArray(_) | CplDataType::CplDict(_)=> {
				//	If the collection is direct (i.e. an array or dictionay) do this
				self.operand_stack.apply_binary_operator_indexed(instruction.block_num, instruction.address, instruction.opcode);
			}


			//	otherwise, get a mutable reference to the collection and then, depending on the type
			//	of collection update the element at the index
			CplDataType::CplVarRef(vr) =>{
				let varref : &mut CplVar = self.operand_stack.fetch_mutable_ref(vr.frame_num, vr.block_num, vr.address);
				match varref.var{
					CplDataType::CplArray (ref mut a) => {
						a.update_indexed_op(&index, &new_value, instruction.opcode);
					}

					CplDataType::CplDict (ref mut d) => {
						d.update_indexed_op(&index, &new_value, instruction.opcode);
					}
					_=> panic!("from exec.apply_binary_operator_indexed:  expected a VarRef to array or dictionary.  Got {}",varref.var),
				}
			}

			_=> panic!("from exec.apply_binary_operator_indexed:  unknown type: {}", var.var),
		}
	}

	/**********************************************************************
	***		Some Unary Operations
	**********************************************************************/
	//	These operators work directly on a variable somewhere in the current
	//	operand frame (note:  if the variable is a VarRef then it can point
	//	to any place in the operand stack)

	//	replace the top of the stack with its length.
	fn exec_length_of(&mut self, ){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_length_of", self.code_block_num, self.instruction_counter)}
		//	get what's at the tos -- don't dereference it as it may he a varref to an array
		let tos = self.operand_stack.pop();
		let length = self.operand_stack.len(&tos);
		self.operand_stack.push(&CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Int, length as f64))));
	}

	fn inc_dec_in_situ(&mut self, opcode : Opcode, frame_num : usize, block_num : usize, address : usize){

		if let CplDataType::CplNumber(ref mut n) = self.operand_stack.operand_frames
					.get_mut(frame_num).unwrap()
					.operand_blocks.get_mut(block_num).unwrap()
					.operand_block.get_mut(address).unwrap()
					.var
		{
			if opcode == Opcode::Inc{
				n.cpl_number += 1.0;
			}else{
				n.cpl_number -= 1.0;
			}
		}else{
			panic!("from inc_dec_in_situ: can only increment or decrement a number");
		}
	}

	//	Increment or Decrement a number.  If the mode is NONE and if the variable at
	//  the top of the stack is NOT a VarRef then replace it with an incremented or
	//	decremented value.  If the value at the top of the stack is a var ref then
	//	act like the mode is Var and remove the VarRef from the stack.  If the mode
	// is Var then increment or decrement in situ.
	fn exec_inc_dec (&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_inc_dec: {}", self.code_block_num, self.instruction_counter, instruction)}
		if instruction.opcode_mode == OpcodeMode::Var{
			self.inc_dec_in_situ(instruction.opcode, self.operand_stack.current_frame(), instruction.block_num, instruction.address);
		}else if instruction.opcode_mode == OpcodeMode::NONE {
			let mut tos = self.operand_stack.pop();
			if let CplDataType::CplVarRef(ref vr) = tos.var{
				self.inc_dec_in_situ(instruction.opcode, vr.frame_num, vr.block_num, vr.address);
			}else if let CplDataType::CplNumber(ref mut n) = tos.var{
				if instruction.opcode == Opcode::Inc{
					n.cpl_number += 1.0;
				}else{
					n.cpl_number -= 1.0;
				}
				self.operand_stack.push(&tos);
			}else{
				panic!("Huston, we have a problem.  Inc/Dec mode is not Var or NONE. It's {}", instruction.opcode_mode);
			}
		}
	}

	fn exec_unary_op(&mut self, instruction : &MachineInstruction){
		self.operand_stack.perform_unary_op(instruction.address, instruction.opcode);
	}

	//	A special purpose opcode used internally for diagnosit purposes
	fn exec_diag(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("**** {}", instruction.literal.token_value)}
		//eprintln!("***** {}",instruction.literal.token_value);
	}

	//	hackability helper.  Get the argument at the top of the stack, clear it and return it.
	fn grab_an_argument(&mut self, _opcode_mode : OpcodeMode) -> CplVar{

		let tos = self.operand_stack.pop();
		match tos.var {
			CplDataType::CplNumber(ref v) => {
				// eprintln!("================ grab_an_argument match number {}, mode={}",v,_opcode_mode);
				return CplVar::new(CplDataType::CplNumber(CplNumber::new(v.rust_data_type, v.cpl_number)))
			},
			CplDataType::CplBool(b) => return CplVar::new(CplDataType::CplBool(CplBool::new(b.cpl_bool))),
			CplDataType::CplString(v) => return CplVar::new(CplDataType::CplString(CplString::new(v.cpl_string.clone()))),
			CplDataType::CplVarRef(v) => {
				// eprintln!("================== grab_an_argument match VarRef {}",v);
				return CplVar::new(CplDataType::CplVarRef(v));
			},
			CplDataType::CplArray(v) => return CplVar::new(CplDataType::CplArray(v.clone())),
			CplDataType::CplUninitialized(_) => return CplVar::new(CplDataType::CplUninitialized(CplUninitialized::new())),
			CplDataType::CplUndefined(_) => return CplVar::new(CplDataType::CplUndefined(CplUndefined::new())),
			CplDataType::CplFileReader(handle) => return CplVar::new(CplDataType::CplFileReader(handle)),
			CplDataType::CplFileWriter(handle) => return CplVar::new(CplDataType::CplFileWriter(handle)),
			CplDataType::CplFileAppender(handle) => return CplVar::new(CplDataType::CplFileAppender(handle)),
			CplDataType::CplDict(d) => return CplVar::new(CplDataType::CplDict(d.clone())),
			CplDataType::CplStruct(s) => return CplVar::new(CplDataType::CplArray(s.clone())),
			//_ => panic!("From grab_an_argument:  I don't understand this {}", tos.var)
		}
	}

	//  A function is called either as a statement or as term in an expression
	//	In either case these are the things we need to do:
	//
	//		1.	push the arguments onto the argument register
	//		2.	push the return address onto return address register
	//
	//	The block_num field of the instruction tells us which frame we're going to
	//	transfer control to.  The address field of the instruction is overloaded with
	//	the number of parameters the called function is expecting.
	//
	fn exec_function_call(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_function_call: {}", self.code_block_num, self.instruction_counter, instruction)}


		//	We don't need this from now on so reset it
		self.arg_count = 0;

		//	grab the arguments
		let mut arguments : Vec<CplVar> = Vec::new();

		let mut arg_counter = 0;

		while arg_counter < instruction.qualifier[0] {
			let arg = self.grab_an_argument(instruction.opcode_mode);
			arguments.push(arg);
			arg_counter += 1;
		}

		//	Now, if the called function is a builtin function, call it directly (somehow),
		//	otherwise we launch a new executor
		if instruction.opcode_mode == OpcodeMode::Builtin{
			//	Write the name of the built-in function being called to the runtime stats
			//	qualifier
			self.runtime_data_qual = instruction.literal.token_value.clone();
			let rslt = (self.builtin_functions.builtin_function_list.get_mut(instruction.block_num).unwrap().target)(&mut self.builtin_functions, &arguments, &mut self.operand_stack);
			if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("      return from Builtin \"{}\" rslt={}", instruction.literal.token_value, rslt)}
			self.operand_stack.push(&rslt);
			return;
		}


		let code_frame_num = instruction.block_num;

		let arguments_len = arguments.len();

		//	instantiate a new exec object
		let mut executor = Executor::call(self.cli
			, self.code_frames
			, code_frame_num
			, &mut arguments
			, &mut self.operand_stack
			, self.builtin_functions
			, arguments_len
			, &mut self.runtime_data
			, self.names,
		);

		//	This is the actual call to the run function
		let return_value = executor.exec();

		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : Return from function call: {} return value {}", self.code_block_num, self.instruction_counter, instruction, return_value)}

		//	If the call was from a statement (as opposed to a term in an expression)
		if instruction.qualifier[1] == 0{
			match return_value.var{
				CplDataType::CplUninitialized(_) => abend!(format!("Call to {} did not return a value",instruction.literal.token_value)),
				_ => {},
			}
		}

		self.operand_stack.push(&return_value);
	}

	fn exec_return(&mut self, _instruction : &MachineInstruction){
		//	get the operand at the top of the current frame
		let operand = self.operand_stack.dereference_tos();

		//	put it into the return value register
		self.return_value = CplVar::new(operand.var);
	}


	//	Allocate (or reuse) a slot in the current block of the current frame.
	fn exec_alloc(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_alloc: {} {} ({})", self.code_block_num, self.instruction_counter, instruction.block_num, instruction.address, self.names[instruction.interner])}
		if self.cli.is_debug_bit(DUMP_OPERANDS){self.dump_operands("at exec_alloc");}
		self.operand_stack.named_alloc(instruction.block_num, instruction.address, instruction.interner);
	}

	/******************************************************************
	*** Jump Instructions
	******************************************************************/
	fn exec_j (&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_j: {}", self.code_block_num, self.instruction_counter, instruction)}

		// if instruction.address >= self.code_frames[self.code_frame_num].code_frame.len() as i32{
		// 	panic! ("From exec_j: address out of bounds {}", instruction.address);
		// }
		self.instruction_counter = instruction.address;
	}

	fn exec_jt(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_jt: {}", self.code_block_num, self.instruction_counter, instruction)}

		let tos = self.operand_stack.dereference_tos();

		match tos.var {
			CplDataType::CplBool(b) => if b.cpl_bool {self.instruction_counter = instruction.address}else{self.instruction_counter+=1},
			CplDataType::CplNumber(s) => if s.cpl_number != 0.0 {self.instruction_counter = instruction.address}else{self.instruction_counter+=1},
			CplDataType::CplString(s) =>  {
				match s.cpl_string.as_str(){
					"true" | "True" | "TRUE" => self.instruction_counter = instruction.address,
					"false" | "False" | "FALSE" => self.instruction_counter += 1,
					_ => {}
				}
			}
			_ => abend!(format!("Frpm exec_jt:  Can't determine truth or falsity of {}", tos.var)),
		}

		if self.cli.is_debug_bit(DUMP_OPERANDS){self.dump_operands("at exec_jt");}
	}

	fn exec_jf(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_jf: {}", self.code_block_num, self.instruction_counter, instruction)}

		let tos = self.operand_stack.dereference_tos();
		match tos.var {
			CplDataType::CplBool(b) => if !b.cpl_bool {self.instruction_counter = instruction.address}else{self.instruction_counter+=1},
			CplDataType::CplNumber(s) => if s.cpl_number == 0.0 {self.instruction_counter = instruction.address}else{self.instruction_counter+=1},
			CplDataType::CplString(s) =>  {
				match s.cpl_string.as_str(){
					"false" | "False" | "FALSE" => self.instruction_counter = instruction.address,
					"true"  | "True"  | "TRUE"  => self.instruction_counter += 1,
					_ => {}
				}
			}
			_ => abend!(format!("From exec_jt:  Can't determine truth or falsity of {}", tos.var)),
		}

		if self.cli.is_debug_bit(DUMP_OPERANDS){self.dump_operands("at exec_jf");}
	}

	//	Branch and Link is a lightweight subroutine call.  Blocks are created
	//	whenever the CPL code starts a new block (e.g. if cond {...}).  The Bl
	//	opcode contains the following information:
	//
	//		-	instruction.address			block end/continue return address
	//		-	instruction.block_num		block end/continue return block number
	//		-	qualifier[0]				if 1 block is breakable else not breakable
	//		-	qualifier[1]				target block number
	//		-	qualifier[2]				break address
	//		-	qualifier[3]				break block number
	//
	//	exec_bl pushes the block/end return info onto the block_end_return_info vector
	//	exec_bl if breakable pushes the break info onto the break_info vector
	//	exec_bl transfers control to block:block_num, address 0
	//
	fn exec_bl(&mut self, instruction : &MachineInstruction){
		//	This is where the BlockEnd instruction jumps to
		self.block_end_return_info.push((if instruction.qualifier[0]>0{true}else{false}, instruction.block_num, instruction.address));

		//	push the break info onto its stack.  Note that the first item in the
		//  tuple indicates whether or not the block is breakable or not.
		self.break_info.push((if instruction.qualifier[0]>0{true}else{false}, instruction.qualifier[2], instruction.qualifier[3]));

		//  set the new block number from the qualifier in the instruction
		self.code_block_num = instruction.qualifier[1];

		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_bl: {}", self.code_block_num, self.instruction_counter, instruction)}
		
		//	start at the first instruction (the instruction counter is incremented
		//	after each instruction so we start at -1 so the next instruction will
		//	be at 0)
		self.instruction_counter = 0;
	}

	//	qualifier[0] of the break instruction contains the number of tokens in
	//	the express following the "break" verb.  If 0 then the default depth
	//	of 1 is used.  If > 0 then the depth is at the tos of the operand stack.
	//	A depth of 0 means pop 1 item from the break_info stack.  1 means pop 2,
	//	and so on.
	//
	//	recall that the Bl instruction contains both return info and break info and
	//	the exec_bl function pushes those data onto their respective stacks.
	fn exec_break (&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_break: depth_flag={}", self.code_block_num, self.instruction_counter, if instruction.qualifier[0]>0{true}else{false})}
		
		let mut depth : usize = 0;
		if instruction.qualifier[0] > 0{
			depth = self.pop_to_number(instruction) as usize;
		}

		let mut depth_counter = depth + 1;
		let mut break_info : (bool,usize,usize) = (false,0,0);


		//	Now pop as many as we need or until we run out
		while ! self.break_info.is_empty() && depth_counter > 0{
			//	get the break info at the top of the break info stack
			break_info = self.break_info.pop().unwrap();

			//	we won't need the return info
			self.block_end_return_info.pop();

			//	reduce the block count by the number of items we pop from the return address stack
			self.block_counter -= 1;

			//	prune the operand stack
			self.operand_stack.pop_block();

			// decrement the loop counter if this is a breakable block
			if break_info.0 {
				depth_counter -= 1;
			}

		}


		//self.dump_operands("at exec_break");

		self.code_block_num = break_info.1;
		self.instruction_counter = break_info.2;

		//if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{} : exec_break:  return to {} block {}", self.instruction_counter, self.code_block_num);}
	}

	//	similar to break, except instead of using "break_info" use block_end_return_info
	fn exec_continue (&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_continue: {}", self.code_block_num, self.instruction_counter, instruction)}

		let mut depth : usize = 0;
		if instruction.qualifier[0] > 0{
			depth = self.pop_to_number(instruction) as usize;
		}

		//	First, get rid of items on the return address stack that were
		//	added to it by Bl (branch and link) instruction until we see one that matches
		//	the jump address in the continue instruction.
		let mut depth_counter = depth + 1;
		let mut block_end_return_info : (bool, usize,usize) = (false,0,0);
		while !self.block_end_return_info.is_empty() && depth_counter > 0{
			block_end_return_info = self.block_end_return_info.pop().unwrap();

			//	don't need this one anymore
			self.break_info.pop();

			//	reduce the block count by the number of items we pop from the return address stack
			self.block_counter -= 1;

			self.operand_stack.pop_block();
			if block_end_return_info.0{
				depth_counter -= 1;
			}
		}

		//	next set the next instruction address from the block and address
		//	numbers in the continue instruction
		self.code_block_num = block_end_return_info.1;
		self.instruction_counter = block_end_return_info.2;
	}

	//	The CPL program level interface to exit.  It no expression was supplied, the default
	//	exit code is 0.  Otherwise it's whatever the expression evaluates to.
	fn exec_exit (&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_exit: {}", self.code_block_num, self.instruction_counter, instruction)}

		let mut exit_code : CplVar = CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Int, 1.0)));
		if instruction.qualifier[0] > 0{
			exit_code = self.operand_stack.dereference_tos();
		}

		self.exit(&exit_code);
	}

	/******************************************************************
	*** Binary operators and support functions
	******************************************************************/

	fn perform_arithmetic_op(&mut self, v1 : f64, v2 : f64, op : Opcode) -> f64{
		match op{
			Opcode::Add=>{v1 + v2}
			Opcode::Sub=>{v1 - v2}
			Opcode::Mul=>{v1 * v2}
			Opcode::Div=>{v1 / v2}
			Opcode::Mod=>{(v1 as i32 % v2 as i32) as f64}
			Opcode::BwAnd=>{(v1 as i32 & v2 as i32) as f64}
			Opcode::BwOr=>{(v1 as i32 | v2 as i32) as f64}
			_=> panic!("From perform_arithmetic_op: expecting a binary arithmetic operator.  Got {}",op),
		}		
	}

	fn perform_numeric_comparison_op(&mut self, v1 : f64, v2 : f64, op : Opcode) -> bool{
		match op{
			Opcode::Lt =>{v1 < v2}
			Opcode::Gt =>{v1 > v2}
			Opcode::Le =>{v1 <= v2}
			Opcode::Ge =>{v1 >= v2}
			Opcode::Eq =>{v1 == v2}
			Opcode::Ne =>{v1 != v2}
			_=> panic!("From perform_numeric_comparison_op: expecting a numeric comparison operator.  Got {}",op),	
		}
	}


	fn perform_string_comparison_op(&mut self, v1 : &str, v2 : &str, op : Opcode) -> bool{
		match op{
			Opcode::Lt =>{v1.len() < v2.len() || (v1.len() == v2.len() && v1 < v2)}
			Opcode::Gt =>{v1.len() > v2.len() || (v1.len() == v2.len() && v1 > v2)}
			Opcode::Le =>{v1.len() <= v2.len() || (v1.len() == v2.len() && v1 <= v2)}
			Opcode::Ge =>{v1.len() >= v2.len() || (v1.len() == v2.len() && v1 >= v2)}
			Opcode::Eq =>{v1.len() == v2.len() && v1 == v2}
			Opcode::Ne =>{v1.len() != v2.len() || v1 != v2}
			_=> panic!("From perform_string_comparison_op: expecting a string comparison operator.  Got {}",op),	
		}
	}

	fn compare_string_string(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		if let CplDataType::CplString(ref v1) = tos1.var{
			if let CplDataType::CplString(ref v2) = tos2.var{
				let rslt : bool = self.perform_string_comparison_op(&v1.cpl_string, &v2.cpl_string, opcode);
				self.operand_stack.push(&CplVar::new(CplDataType::CplBool(CplBool::new(rslt))));
			}
		}
	}
	fn compare_string_number(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		//	To compare strings and numbers, we try to convert the string to a number
		//	then compare.  If the string isn't a number then we know the comparison
		//	must be false
		if let CplDataType::CplString(ref v1) = tos1.var{
			match v1.cpl_string.parse::<f64>() {
				Err(_) => self.operand_stack.push(&CplVar::new(CplDataType::CplBool(CplBool::new(false)))),
				Ok(parsed) => if let CplDataType::CplNumber(ref v2) = tos2.var {
					let rslt = self.perform_numeric_comparison_op(parsed, v2.cpl_number, opcode);
					self.operand_stack.push(&CplVar::new(CplDataType::CplBool(CplBool::new(rslt))));
				}
			}
		}
	}

	fn compare_string_bool(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		//	the string must be either "true" or "false".  If not then the result
		//	must be false.  Only equality is valid
		if opcode != Opcode::Eq {
			panic!("from compare_string_bool: Invalid operation {}{}{}", tos1, opcode, tos2);
		}
		let mut rslt : bool = false;
		if let CplDataType::CplString(ref v1) = tos1.var{
			if let CplDataType::CplBool(ref v2) = tos2.var{
				match &v1.cpl_string as &str{
					"false" => if v2.cpl_bool { rslt = false } else { rslt = true },
					"true"  => if v2.cpl_bool { rslt = true } else { rslt = false },
					_=> {rslt = false},
				}
			}
		}
		self.operand_stack.push(&CplVar::new(CplDataType::CplBool(CplBool::new(rslt))));
	}

	fn compare_number_string(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		//	To compare a number with a string, we try to conver the string to a
		//	number then compare.  if it doesn't convert we return false
		if let CplDataType::CplString(ref v2) = tos2.var{
			match v2.cpl_string.parse::<f64>(){
				Err(_) => self.operand_stack.push(&CplVar::new(CplDataType::CplBool(CplBool::new(false)))),
				Ok(parsed) => if let CplDataType::CplNumber(ref v1) = tos1.var {
					let rslt = self.perform_numeric_comparison_op(v1.cpl_number, parsed, opcode);
					self.operand_stack.push(&CplVar::new(CplDataType::CplBool(CplBool::new(rslt))));
					return;
				}
			}
		}
		self.operand_stack.push(&CplVar::new(CplDataType::CplBool(CplBool::new(false))));
	}

	fn compare_number_number(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		if let CplDataType::CplNumber(ref v1) = tos1.var{
			if let CplDataType::CplNumber(ref v2) = tos2.var{
				let rslt = self.perform_numeric_comparison_op(v1.cpl_number, v2.cpl_number, opcode);
				self.operand_stack.push(&CplVar::new(CplDataType::CplBool(CplBool::new(rslt))));
				return;
			}
		}
		self.operand_stack.push(&CplVar::new(CplDataType::CplBool(CplBool::new(false))));
	}

	fn compare_number_bool(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		//	if the number is either 1 or 0 then we can compare it to true and false
		//	respectively otherwise the result is false.  Only equality is valid
		if opcode != Opcode::Eq{
			panic!("from compare_number_bool:  Invalid expression: {} {} {}",tos1, opcode, tos2);
		}
		let mut rslt : bool = false;
		if let CplDataType::CplNumber(ref v1) = tos1.var{
			if let CplDataType::CplBool(ref v2) = tos2.var{
				if v1.cpl_number == 0.0 && v2.cpl_bool == true{rslt = false}
				if v1.cpl_number == 0.0 && v2.cpl_bool == false{rslt = true}
				if v1.cpl_number == 1.0  && v2.cpl_bool == true {rslt = true}
				if v1.cpl_number == 1.0 && v2.cpl_bool == false {rslt = false}
			}
		}
		self.operand_stack.push(&CplVar::new(CplDataType::CplBool(CplBool::new(rslt))));
	}


	fn compare_bool_string(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		//	To compare a bool with a string, the string must be either "true" or "false"
		//	if it isn't then the result will be false.  Only equality is valid.
		if opcode != Opcode::Eq{
			panic!("from compare_bool_string:  Invalid expression: {} {} {}",tos1, opcode, tos2);
		}
		let mut rslt : bool = false;
		if let CplDataType::CplString(ref v2) = tos2.var{
			if let CplDataType::CplBool(ref v1) = tos1.var{
				match &v2.cpl_string as &str{
					"false" => if v1.cpl_bool { rslt = false } else { rslt = true },
					"true"  => if v1.cpl_bool { rslt = true } else { rslt = false },
					_=> {rslt = false},
				}
			}
		}
		self.operand_stack.push(&CplVar::new(CplDataType::CplBool(CplBool::new(rslt))));
		return;
	}
	
	fn compare_bool_number(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		//	To compare bool against a number, the number must be either 0 or 1 and
		//	if it isn't the result will be false.  Only equality is valid
		if opcode != Opcode::Eq{
			panic!("from compare_bool_number:  Invalid expression: {} {} {}",tos1, opcode, tos2);
		}

		let mut rslt : bool = false;
		if let CplDataType::CplNumber(ref v2) = tos2.var{
			if let CplDataType::CplBool(ref v1) = tos1.var{
				if v2.cpl_number == 0.0 && v1.cpl_bool == true{rslt = false}
				if v2.cpl_number == 0.0 && v1.cpl_bool == false{rslt = true}
				if v2.cpl_number == 1.0 && v1.cpl_bool == true {rslt = true}
				if v2.cpl_number == 1.0 && v1.cpl_bool == false {rslt = false}
			}
		}
		self.operand_stack.push(&CplVar::new(CplDataType::CplBool(CplBool::new(rslt))));
	}
	fn compare_bool_bool(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		// Only equality is valid
		if opcode != Opcode::Eq{
			panic!("from compare_bool_number:  Invalid expression: {} {} {}",tos1, opcode, tos2);
		}
				
		if let CplDataType::CplBool(ref v1) = tos1.var{
			if let CplDataType::CplBool(ref v2) = tos2.var{
				if v1.cpl_bool == v2.cpl_bool{
					self.operand_stack.push(&CplVar::new(CplDataType::CplBool(CplBool::new(true))));
				}else{
					self.operand_stack.push(&CplVar::new(CplDataType::CplBool(CplBool::new(false))));
				}
				return;
			}
		}
		self.operand_stack.push(&CplVar::new(CplDataType::CplBool(CplBool::new(false))));
	}

	fn do_op_string_string(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		//	If both strings can be converted to numbers, then do_op_number_number, otherwise
		//	it's only "." works  But if the opcode is "." then this only works with strings
		if let CplDataType::CplString(ref v1) = tos1.var{
			if let CplDataType::CplString(ref v2) = tos2.var{
				if opcode != Opcode::Concat{
					match v1.cpl_string.parse::<f64>() {
						Err(_) => {},
						Ok (parsed1) => {
							match v2.cpl_string.parse::<f64>() {
								Err(_) => {},
								Ok (parsed2) => {
									let rslt = self.perform_arithmetic_op(parsed1, parsed2, opcode);
									self.operand_stack.push(&CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Real, rslt))));
									return;
								}
							}
						}
					}
				}

				if opcode == Opcode::Concat{
					let mut rslt = String::new();
					rslt.push_str(&v1.cpl_string);
					rslt.push_str(&v2.cpl_string);
					self.operand_stack.push(&CplVar::new(CplDataType::CplString(CplString::new(rslt))));
					return;
				}
			}
		}

		panic!("from do_op_string_string: Invalid expression {} {} {}",tos1,opcode,tos2);
	}

	fn do_op_string_number(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		//	To do this, we try to convert the string to a number and, if it conversts,
		//	we can do the arithmetic operation.  If it doesn't we convert the number
		//	to a string and then do string_string.
		
		if let CplDataType::CplString(ref v1) = tos1.var{
			if let CplDataType::CplNumber(ref v2) = tos2.var{
				match v1.cpl_string.parse::<f64>(){
					Err(_) => if opcode == Opcode::Concat || opcode == Opcode::Add{
						let mut rslt = String::new();
						rslt.push_str(&v1.cpl_string);
						rslt.push_str(&v2.cpl_number.to_string());
						self.operand_stack.push(&CplVar::new(CplDataType::CplString(CplString::new(rslt))));
					},
				
					Ok(parsed) => {
						let rslt = self.perform_arithmetic_op(parsed, v2.cpl_number, opcode);
						self.operand_stack.push(&CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Real, rslt))));
					}
				}
				return;
			}
		}
		panic!("from do_op_string_number: Invalid expression {} {} {}",tos1,opcode,tos2);
	}

	fn do_op_string_bool(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		//	there are not operations that work on a string and boolean
		panic!("from do_op_string_number: Invalid expression {} {} {}",tos1,opcode,tos2);
	}

	fn do_op_string_array(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		//	opcode:concat creates a string version of the array content in tos2 and appends
		//	that string to tos1.  no other opcodes are supported at this time
		if opcode != Opcode::Concat {
			panic!("from do_op_string_array: Invalid expression {} {} {}",tos1,opcode,tos2);
		}
		let rslt = format!("{}[{}]",tos1,tos2);
		self.operand_stack.push(&CplVar::new(CplDataType::CplString(CplString::new(rslt))));
	}

	fn do_op_number_string(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		//	same as string_number
		if let CplDataType::CplString(ref v2) = tos2.var{
			if let CplDataType::CplNumber(ref v1) = tos1.var{
				match v2.cpl_string.parse::<f64>(){
					Err(_) => if opcode == Opcode::Concat || opcode == Opcode::Add{
						let mut rslt = String::new();
						rslt.push_str(&v1.cpl_number.to_string());
						rslt.push_str(&v2.cpl_string);
						self.operand_stack.push(&CplVar::new(CplDataType::CplString(CplString::new(rslt))));
					},
				
					Ok(parsed) => {
						let rslt = self.perform_arithmetic_op(parsed, v1.cpl_number, opcode);
						self.operand_stack.push(&CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Real, rslt))));
					}
				}
				return;
			}
		}
		panic!("from do_op_number_string: Invalid expression {} {} {}",tos1,opcode,tos2);
	}

	fn do_op_number_number(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		if let CplDataType::CplNumber(ref v2) = tos2.var{
			if let CplDataType::CplNumber(ref v1) = tos1.var{
				let rslt = self.perform_arithmetic_op(v1.cpl_number, v2.cpl_number, opcode);
				self.operand_stack.push(&CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Real, rslt))));
				return;
			}
		}
		panic!("from do_op_number_number: Invalid expression {} {} {}",tos1,opcode,tos2);
	}
	fn do_op_number_bool(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		//	there are not operations that work on a number and boolean
		panic!("from do_op_number_bool: Invalid expression {} {} {}",tos1,opcode,tos2);
	}

	fn do_op_number_array(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		//	there are not operations that work on a number and boolean
		panic!("from do_op_number_array: Invalid expression {} {} {}",tos1,opcode,tos2);
	}


	fn do_op_bool_string(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		//	there are not operations that work on a bool and string
		panic!("from do_op_bool_string: Invalid expression {} {} {}",tos1,opcode,tos2);
	}

	fn do_op_bool_number(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		panic!("from do_op_bool_number: Invalid expression {} {} {}",tos1,opcode,tos2);
	}

	fn do_op_bool_bool(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		panic!("from do_op_bool_bool: Invalid expression {} {} {}",tos1,opcode,tos2);
	}

	fn do_op_bool_array(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		panic!("from do_op_bool_array: Invalid expression {} {} {}",tos1,opcode,tos2);
	}

	//	The folllowing array operations only support Push and Append.  Things like
	//	updating values in an array with some assignment operator or creating the
	//	product to two vectors we'll leave for another day.
	fn do_op_array_string(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		panic!("from do_op_array_string: Invalid expression {} {} {}",tos1,opcode,tos2);
		//	opcode::concat is replaces Push built-in
		// if let CplDataType::CplVarRef(var_ref) = &tos1.var{
		// 	let operand_frames = self.operand_stack.operand_frames.last_mut().unwrap();
		// 	if let CplDataType::CplArray(array) = &mut operand_frames.operand_blocks[var_ref.block_num].operand_block[var_ref.address].var{
		// 		array.push(&new_value);
		// 		return CplVar::new(CplDataType::CplUninitialized(CplUninitialized::new()));
		// 	}else{
		// 		panic!("From builtin_push - Push:  Expecting parameter 1 to be a reference to an array (e.g. Push(&array,item_to_push))")	
		// 	}
		// }else{
		// 	panic!("From builtin_push - Push:  Expecting parameter 1 to be a reference to an array (e.g. Push(&array,item_to_push))")
		// }
	}
	fn do_op_array_number(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		//	opcode::concat pushes tos2 onto tos1, rest are invalid
		panic!("from do_op_array_number: Invalid expression {} {} {}",tos1,opcode,tos2);
	}
	fn do_op_array_bool(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		//	opcode::concat pushs tos2 onto tos1, rest are invalid
		panic!("from do_op_array_bool: Invalid expression {} {} {}",tos1,opcode,tos2);
	}
	fn do_op_array_array(&mut self,tos1 : &CplVar,tos2 : &CplVar,opcode : Opcode){
		//	opcode::concat replaces the Append built-in creating a new
		//	array consisting of the two arrays, rest are invalid
		panic!("from do_op_array_array: Invalid expression {} {} {}",tos1,opcode,tos2);
	}


	fn operand_eval(&self, operand1 : &CplVar, operand2 : &CplVar) -> OperandAnalysis{		
		let t1 = match operand1.var{
			CplDataType::CplString(_) => OperandType::OtString,
			CplDataType::CplNumber(_) => OperandType::OtNumber,
			CplDataType::CplBool(_) => OperandType::OtBool,
			CplDataType::CplArray(_) => OperandType::OtArray,
			_ => return OperandAnalysis::InvalidType,
		};

		let t2 = match operand2.var{
			CplDataType::CplString(_) => OperandType::OtString,
			CplDataType::CplNumber(_) => OperandType::OtNumber,
			CplDataType::CplBool(_) => OperandType::OtBool,
			CplDataType::CplArray(_) => OperandType::OtArray,
			_ => return OperandAnalysis::InvalidType,
		};

		if t1==OperandType::OtString && t2==OperandType::OtString{
			return OperandAnalysis::StringString;
		}

		if t1==OperandType::OtString && t2==OperandType::OtNumber{
			return OperandAnalysis::StringNumber
		}
		
		if t1==OperandType::OtString && t2==OperandType::OtBool{
			return OperandAnalysis::StringBool;
		}

		if t1==OperandType::OtString && t2==OperandType::OtArray{
			return OperandAnalysis::StringArray;
		}

		if t1==OperandType::OtNumber && t2==OperandType::OtString{
			return OperandAnalysis::NumberString;
		}
		if t1==OperandType::OtNumber && t2==OperandType::OtNumber{
			return OperandAnalysis::NumberNumber;
		}
		if t1==OperandType::OtNumber && t2==OperandType::OtBool{
			return OperandAnalysis::NumberBool;
		}

		if t1==OperandType::OtNumber && t2==OperandType::OtArray{
			return OperandAnalysis::NumberArray;
		}

		if t1==OperandType::OtBool && t2==OperandType::OtString{
			return OperandAnalysis::BoolString
		}
		if t1==OperandType::OtBool && t2==OperandType::OtNumber{
			return OperandAnalysis::BoolNumber;
		}
		if t1==OperandType::OtBool && t2==OperandType::OtBool{
			return OperandAnalysis::BoolBool;
		}

		if t1==OperandType::OtBool && t2==OperandType::OtArray{
			return OperandAnalysis::BoolArray;
		}
		
		if t1==OperandType::OtArray && t2==OperandType::OtString{
			return OperandAnalysis::ArrayString
		}
		if t1==OperandType::OtArray && t2==OperandType::OtNumber{
			return OperandAnalysis::ArrayNumber;
		}
		if t1==OperandType::OtArray && t2==OperandType::OtBool{
			return OperandAnalysis::ArrayBool;
		}
		if t1==OperandType::OtArray && t2==OperandType::OtArray{
			return OperandAnalysis::ArrayArray;
		}
		panic!("Huston, we have a problem.  From operand_eval: combinations accounted for");
	}

	fn exec_binary_operator(&mut self, instruction : &MachineInstruction){
		//	get the two values to compare from the operand stack
		let tos2 = self.operand_stack.dereference_tos();
		let tos1 = self.operand_stack.dereference_tos();

		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_binary_operator({}): (tos1={} tos2={})", self.code_block_num, self.instruction_counter, instruction.opcode, tos1.dbg(), tos2.dbg())}

		let eval = self.operand_eval(&tos1, &tos2);
		match instruction.opcode{
			Opcode::Lt | Opcode::Gt | Opcode::Le | Opcode::Ge | Opcode::Eq | Opcode::Ne => {
				match eval{
					OperandAnalysis::StringString 		=>	self.compare_string_string(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::StringNumber		=>	self.compare_string_number(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::StringBool			=>	self.compare_string_bool(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::NumberString		=>	self.compare_number_string(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::NumberNumber		=>	self.compare_number_number(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::NumberBool			=>	self.compare_number_bool(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::BoolString			=>	self.compare_bool_string(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::BoolNumber			=>	self.compare_bool_number(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::BoolBool			=>	self.compare_bool_bool(&tos1,&tos2,instruction.opcode),
					_=> if self.cli.is_runtime_warnings(){
						eprintln!("WARNING from exec_binary_operator: {}{}{} is invalid.  returning false", tos1, instruction.opcode, tos2);
						self.operand_stack.push(&CplVar::new(CplDataType::CplBool(CplBool::new(false))));
					}
				}		
			}
			Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Div | Opcode::Mod | Opcode::Concat | Opcode::BwAnd | Opcode::BwOr =>{
				match eval{
					OperandAnalysis::StringString 		=>	self.do_op_string_string(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::StringNumber		=>	self.do_op_string_number(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::StringBool			=>	self.do_op_string_bool(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::StringArray		=>	self.do_op_string_array(&tos1,&tos2,instruction.opcode),

					OperandAnalysis::NumberString		=>	self.do_op_number_string(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::NumberNumber		=>	self.do_op_number_number(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::NumberBool			=>	self.do_op_number_bool(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::NumberArray		=>	self.do_op_number_array(&tos1,&tos2,instruction.opcode),

					OperandAnalysis::BoolString			=>	self.do_op_bool_string(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::BoolNumber			=>	self.do_op_bool_number(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::BoolBool			=>	self.do_op_bool_bool(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::BoolArray			=>	self.do_op_bool_array(&tos1,&tos2,instruction.opcode),

					OperandAnalysis::ArrayString		=>	self.do_op_array_string(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::ArrayNumber		=>	self.do_op_array_number(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::ArrayBool			=>	self.do_op_array_bool(&tos1,&tos2,instruction.opcode),
					OperandAnalysis::ArrayArray			=>	self.do_op_array_array(&tos1,&tos2,instruction.opcode),
					_=> panic!("from exec_binary_operator: {}{}{} is invalid", tos1, instruction.opcode, tos2),
				}		
			}
			_ => panic!("from exec_binary_operator:  I can't do anything with this opcode: {}",instruction.opcode),

		}
	}


	fn exec_lor_land(&mut self, instruction : &MachineInstruction){
		let tos2 = self.operand_stack.dereference_tos();
		let tos1 = self.operand_stack.dereference_tos();

		if let CplDataType::CplBool(ref b1) = tos1.var{
			if let CplDataType::CplBool(ref b2) = tos2.var{
				if instruction.opcode == Opcode::Lor {
					self.operand_stack.push(&CplVar::new(CplDataType::CplBool(CplBool::new(b1.cpl_bool || b2.cpl_bool))));
				} else if instruction.opcode == Opcode::Land {
					self.operand_stack.push(&CplVar::new(CplDataType::CplBool(CplBool::new(b1.cpl_bool && b2.cpl_bool))));
				}else{
					panic!("from exec_loc_land: Expending '||' or '&&' got: {}",instruction.opcode)
				}
			}
		}
	}

	fn foreach_index (&self, index_block_num : usize, index_address :usize) -> usize{
		let index : usize;
		//	fetch the index and increment it
		let index_var = self.operand_stack.fetch_local_ref(index_block_num, index_address);
		if let CplDataType::CplNumber(ref n) = index_var.var{
			index = n.cpl_number as usize;
			return index;
		}else{
			return 0;
		}
	}

	fn foreach_index_check(&self, index : usize, collection_block_num : usize, collection_address : usize) -> bool{
		let collection_ref = self.operand_stack.fetch_local_ref(collection_block_num, collection_address);
		if let CplDataType::CplArray(ref collection) = collection_ref.var{
			if index >= collection.cpl_array.len(){
				return false;
			}else{
				return true;
			}
		}
		return false;
	}

	fn foreach_index_increment (&mut self, index_block_num : usize, index_address : usize){
		match self.operand_stack.fetch_local_mutable_ref(index_block_num, index_address).var{
			CplDataType::CplNumber(ref mut n) => {
				n.cpl_number += 1.0;
			}
			_ => {}
		}
	}

	fn foreach_target_update (&mut self
			, index : usize
			, collection_block_num : usize
			, collection_address : usize
			, target_block_num : usize
			, target_address : usize
			)
			{

		let collection_element = self.operand_stack.fetch_indexed(index, collection_block_num, collection_address);
		self.operand_stack.update_local(&collection_element, target_block_num, target_address);
	}

	//	Foreach:  Loop in a box
	//
	//	Taking some liberal advantage of the fields in an instruction:
	//
	//		Block_num = block number of target
	// 		Address = address of target
	// 		Qual[0] = Loop exit address when index >= length of array
	// 		Qual[1] = block_num of index
	// 		Qual[2] = address of index
	// 		Qual[3] = block_num of array
	// 		Qual[4] = address of array

	fn exec_foreach(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){eprintln!("{}:{} : exec_foreach: {}", self.code_block_num, self.instruction_counter, instruction)}

		let target_block_num = instruction.block_num;
		let target_address = instruction.address;
		let index_block_num = instruction.qualifier[1];
		let index_address = instruction.qualifier[2];
		let collection_block_num = instruction.qualifier[3];
		let collection_address = instruction.qualifier[4];
		let loop_exit_address = instruction.qualifier[0];

		let index = self.foreach_index (index_block_num, index_address);
		if !self.foreach_index_check(index, collection_block_num, collection_address){
			self.instruction_counter = loop_exit_address;
			return;
		}

		self.foreach_index_increment(index_block_num, index_address);

		self.foreach_target_update(index, collection_block_num, collection_address, target_block_num, target_address);

		//	since the executor won't do this for foreach instructinos
		self.instruction_counter += 1;
	}
}