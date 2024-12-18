
#![allow(unused_imports)]

use tokenizer::*;
use cli::*;
use cplvar::*;
use builtin::*;
use opcode::*;
use machineinstruction::*;
use codeframe::*;
//use abend::*;
use macrolib::*;

// fn undefined()->CplVar{
// 	CplVar::new(CplDataType::CplUndefined(CplUndefined::new()))
// }

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
	block_end_return_address : Vec<(usize, usize)>,

	//	All the currently active variables and the frame numbers
	//	An operand frame is created whenever the executor is
	//	created (i.e. whenever a function is called) and is
	//	destoryed when control returns to a callling function
	operand_stack : & 'a mut OperandStack,


	//	The arguments that were passed to this function by a calling
	//	function
	arguments : & 'a mut Vec<CplVar>,		// arguments passed to this function

	//	A reference to the external function table which was
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
}

impl<'a> Executor<'a>{
	pub fn new(
				cli : & 'a CLI<'a>
				, frame_map : & 'a mut FrameMap
				, arguments : & 'a mut Vec<CplVar>
				, operand_stack : & 'a mut OperandStack
				, arg_count : usize
	    	) -> Executor<'a> {
		Executor{
			cli : cli,
			code_frames : &frame_map.frames_list,
			code_frame_num : frame_map.get_entry_frame_number(),
			code_block_num : 0,				// always start at the first code block
			block_end_return_address : Vec::new(),
			//return_address : Vec::new(),
			operand_stack : operand_stack,
			builtin_functions : &mut frame_map.builtin_function_table,
			arguments : arguments,

			arg_count : arg_count,
			instruction_counter : 0,
			return_value : CplVar::new(CplDataType::CplUninitialized(CplUninitialized::new())),
			block_counter : 0,
			_call_flag : false,
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
		) -> Executor<'a> {
	
		Executor{
			cli : cli,
			code_frames : code_frames,
			code_frame_num : code_frame_num,
			code_block_num : 0,				// always start at the first code block
			block_end_return_address : Vec::new(),
			//return_address : Vec::new(),
			operand_stack : operand_stack,
			builtin_functions : builtin_functions,
			arguments : arguments,

			arg_count : arg_count,
			instruction_counter : 0,
			return_value : CplVar::new(CplDataType::CplUninitialized(CplUninitialized::new())),
			block_counter : 0,
			_call_flag : true,
		}
	}

	fn dump_operands(&self, title : &str){
		println!("{}", title);
		self.operand_stack.dump_operands();
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

		while self.instruction_counter < self.code_frames[self.code_frame_num].code_block_list[self.code_block_num].code_block.len() {
			//	get the current instruction
			let instruction = self.code_frames[self.code_frame_num].code_block_list[self.code_block_num].code_block.get(self.instruction_counter).unwrap();
			if self.cli.is_debug_bit(TRACE_EXEC_DISPATCHER){println!("exec(dispatcher): {}:{} {}",self.code_block_num, self.instruction_counter, instruction);}
			
			match instruction.opcode{

				Opcode::BlockBegin 				=> self.exec_block_begin(instruction),
				Opcode::BlockEnd				=> self.exec_block_end(instruction),
				Opcode::Push 					=> self.exec_push(instruction),
				Opcode::PushNewCollection		=> self.exec_push_new_collection(instruction),
				Opcode::Print 					=> self.exec_print(instruction.clone()),
				Opcode::Pop 					=> self.exec_pop(instruction),

				Opcode::Add 					=> self.exec_add(instruction),
				Opcode::Sub 					=> self.exec_sub(instruction),
				Opcode::Mul 					=> self.exec_mul(instruction),
				Opcode::Div 					=> self.exec_div(instruction),
				Opcode::Mod 					=> self.exec_mod(instruction),

				Opcode::AddEq 					=> self.exec_assignment_operator(instruction),
				Opcode::SubEq 					=> self.exec_assignment_operator(instruction),
				Opcode::MulEq 					=> self.exec_assignment_operator(instruction),
				Opcode::DivEq					=> self.exec_assignment_operator(instruction),
				Opcode::ModEq 					=> self.exec_assignment_operator(instruction),
				Opcode::OrEq 					=> self.exec_assignment_operator(instruction),
				Opcode::AndEq 					=> self.exec_assignment_operator(instruction),
				Opcode::AppendEq 				=> self.exec_assignment_operator(instruction),

				Opcode::Update					=> self.exec_update(instruction),
	

				Opcode::Lor 					=> self.exec_lor(instruction),
				Opcode::Land 					=> self.exec_land(instruction),
				Opcode::BwOr 					=> self.exec_bw_or(instruction),
				Opcode::BwAnd 					=> self.exec_bw_and(instruction),

				Opcode::Inc 					=> self.exec_unary_op(instruction),
				Opcode::Dec 					=> self.exec_unary_op(instruction),
				Opcode::Uminus 					=> self.exec_unary_op(instruction),	
				Opcode::Damnit					=> self.exec_unary_op(instruction),

				Opcode::Concat 					=> self.exec_concat(instruction),

				Opcode::J						=> self.exec_j(instruction),
				Opcode::Jt						=> self.exec_jt(instruction),
				Opcode::Jf						=> self.exec_jf(instruction),
				Opcode::Bl						=> self.exec_bl(instruction),

				Opcode::Break					=> self.exec_break(instruction),
				Opcode::Continue				=> self.exec_continue(instruction),

				Opcode::Lt						=> self.exec_lt(instruction),
				Opcode::Gt						=> self.exec_gt(instruction),
				Opcode::Le						=> self.exec_le(instruction),
				Opcode::Ge						=> self.exec_ge(instruction),
				Opcode::Ne						=> self.exec_ne(instruction),
				Opcode::Eq						=> self.exec_eq(instruction),
	
				Opcode::FunctionCall			=> self.exec_function_call(instruction),
				Opcode::FetchIndexed			=> self.exec_fetch_indexed(instruction),

				Opcode::IncArgCount 			=> self.arg_count += 1,
				Opcode::Return					=> self.exec_return(instruction),

				Opcode::Alloc					=> self.exec_alloc(instruction),

				Opcode::Diag					=> self.exec_diag(instruction),

				_ => abend!(format!("{} Not Implemented Yet", instruction.opcode)),
			}

			if self.cli.is_debug_bit(DUMP_OPERANDS_DISPATCH){
				println! ("instruction: {}", instruction);
				self.dump_operands("After instruction Execution");
			}

			//	Don't increment if the instruction counter was set by Retrun, Bl, Break or a jump of some kind
			//	because those opcodes set the next address to get the instruction from rather than
			//	the next opcode in the instruction stream.  Return is kind of a special case because
			//	if we're in the entry function, we break out of the execution loop.
			match instruction.opcode{
				Opcode::Bl |
				Opcode::J  |
				Opcode::Jt |
				Opcode::Jf |
				Opcode::BlockEnd |
				Opcode::Break |
				Opcode::Continue		=> {},
				
				//	After executing the return instruction which adds the return
				//	value to return value register we break out of the exec loop
				//	because we're done with the function.  Recall that an active
				//	function is an instantiation of the executor object
				Opcode::Return			=> break,

				_						=> self.instruction_counter += 1,
			}
		}


		if self.cli.is_debug_bit(TRACE_EXEC_DISPATCHER){println!("exec(dispatcher -- exit): {}:{}",self.code_block_num, self.instruction_counter);}
		self.operand_stack.pop_frame();
		return CplVar::new(self.return_value.var.clone());
	}

	//	create a new operand block (e.g. for a WHILE instruction) in the current frame
	//	I can't think of reason to support creating a block in another frame
	fn exec_block_begin(&mut self, instruction : &MachineInstruction){		
		if self.cli.is_debug_bit(TRACE_EXEC){
			println!("{}:{} : exec_block_begin: {} block {}", self.code_block_num, self.instruction_counter, instruction, self.block_counter);
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

		if self.block_end_return_address.is_empty(){
			if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_block_end: {} block_len {}", self.code_block_num,self.instruction_counter, instruction, self.operand_stack.variable_count())}
			self.instruction_counter += 1;
			return;
		}

		let block_end_return_address = self.block_end_return_address.pop().unwrap();

		if self.cli.is_debug_bit(TRACE_EXEC){
			println!("{}:{} : exec_block_end: {} returning to: {}:{}", 
				self.code_block_num, self.instruction_counter, self.code_block_num, block_end_return_address.0, block_end_return_address.1);
		}

		self.code_block_num = block_end_return_address.0;
		self.instruction_counter = block_end_return_address.1;


		self.block_counter -= 1;


		//	pop the operand stack frame
		self.operand_stack.pop_block();
	}

	//	Fetch a value from an array or dictionary which is either at the
	//	top of the stack or at a location pointed to by a VarRef at the
	//	top of the stack.  Note that in some cases, there may be two
	//	levels of indirection.  This will occur when


	//	Fetch a value from an array or dictionary at the top of the operand stack using an
	//	index which is also on the operand stack.  Push the retreived value onto the operand
	//	stack. The retrieved value may be a refernce to a variable.
	fn exec_fetch_indexed(&mut self, instruction: &MachineInstruction){
		let fetched = self.operand_stack.fetch_indexed_from_operand_stack();
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_fetch_indexed: {} got: {}", self.code_block_num, self.instruction_counter, instruction, fetched)}
		self.operand_stack.push(&fetched);

	}

	//	Push instructions always operate on the current top of the operand stack (i.e.
	//	the current operand frame)

	fn push_lit_string(&mut self, instruction: &MachineInstruction, _instruction_address : usize){
		//	get the current operand_frame
		if self.cli.is_debug_bit(TRACE_EXEC){println!("     push_lit_string: {}", instruction)}

		self.operand_stack.push(&CplVar::new(
			CplDataType::CplString(CplString::new(instruction.literal.token_value.clone()))));
	}

	fn push_lit_number(&mut self, instruction: &MachineInstruction, _instruction_address : usize){
		//	get the current operand_frame
		if self.cli.is_debug_bit(TRACE_EXEC){println!("      push_lit_number: {}", instruction)}

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
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : push_lit_bool_help: {}", self.code_block_num,instruction_address, instruction)}
		self.operand_stack.push(&CplVar::new(CplDataType::CplBool (CplBool::new (b))));
	}

	//	used by exec_push
	fn push_lit_bool(&mut self, instruction: &MachineInstruction, instruction_address : usize){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : push_lit_bool: {}", self.code_block_num, instruction_address, instruction)}
		let b = instruction.literal.token_value.parse::<bool>().unwrap();
		self.push_lit_bool_help(b, instruction_address, instruction);
	}

	//	used by exec_push to create an uninitialized variable
	fn push_lit_none(&mut self, instruction: &MachineInstruction){
		//	get the current operand_frame
		if self.cli.is_debug_bit(TRACE_EXEC){println!("     push_lit_none: {}", instruction)}

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
			println!("push_lit: {}", instruction.literal.token_type);
		}
	}

	//	If the operand is a scalar (e.g. number) or a VarRef then just push it onto the
	//	stack.  If the operand is a collection, create a VarRef pointing at it
	//	and push that onto the stack.
	fn push_copy(&mut self, operand : &CplVar, instruction : &MachineInstruction){
		// if self.cli.is_debug_bit(TRACE_EXEC){println!("      push_copy: operand={}, instruction={}",operand, instruction)}
		//println!("=========== from push_copy {} {}", operand, instruction);
		match operand.var{
			CplDataType::CplNumber(_)			|
			CplDataType::CplString(_)			|
			CplDataType::CplBool(_) 			|
			CplDataType::CplVarRef(_)			|
			CplDataType::CplUninitialized(_)	|
			CplDataType::CplUndefined(_) 		=> {
				if instruction.opcode_mode == OpcodeMode::VarRef{
					if self.cli.is_debug_bit(TRACE_EXEC){println!("      push_copy(mode={}) {},{},{}", instruction.opcode_mode, self.operand_stack.current_frame(), instruction.block_num, instruction.address)};
					self.operand_stack.push(&CplVar::new(CplDataType::CplVarRef(CplVarRef::new(self.operand_stack.current_frame(), instruction.block_num, instruction.address))));
				}else{
					if self.cli.is_debug_bit(TRACE_EXEC){println!("      push_copy(mode={}) {}", instruction.opcode_mode, instruction)};
					self.operand_stack.push(operand);
				}
			}
			
			CplDataType::CplArray(_) | CplDataType::CplDict(_) =>{
				if self.cli.is_debug_bit(TRACE_EXEC){println!("      push_copy(Collection) {}", instruction)};
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
		if self.cli.is_debug_bit(TRACE_EXEC){println!("      push_arg: {}", instruction)}
		//	fetch the next argument from the arguments register
		let arg = self.arguments.pop().unwrap();
		//	and push it onto the stack
		self.operand_stack.push(&arg);
	}

	//	Push Array means:  create a new empty array at the top of the stack
	// fn push_array(&mut self, instruction : &MachineInstruction, _instruction_address : usize){
	// 	if self.cli.is_debug_bit(TRACE_EXEC){println!("     push_array: {}",instruction)}
	// 	self.operand_stack.push(&CplVar::new(CplDataType::CplArray(CplArray::new())));
	// }

	// //	Push Dict means:  create a new empty dictionary rray at the top of the stack
	// fn push_dict(&mut self, instruction : &MachineInstruction, instruction_address : usize){
	// 	if self.cli.is_debug_bit(TRACE_EXEC){println!("     push_dict: {}", self.code_block_num, instruction_address, instruction)}
	// 	self.operand_stack.push(&CplVar::new(CplDataType::CplDict(CplDict::new())));
	// }


	//	Create a new collection on the opernand stack.  The mode says what kind of
	//	collection.  This will be the only actual instance of the collection.  All other
	//	"instances" will be VarRef's to it (I hope).
	fn exec_push_new_collection(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_push_new_collection: {} Mode: {}", self.code_block_num, self.instruction_counter,instruction, instruction.opcode_mode)}
		match instruction.opcode_mode{
			OpcodeMode::Array		=> {
				self.operand_stack.push(&CplVar::new(CplDataType::CplArray(CplArray::new())));
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
	//		  (this is used for struct instantiation -- see exec_update (mode = array)
	//		  which does something similar -- maybe even the same thing)
	fn exec_push(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_push: {} Mode: {}", self.code_block_num, self.instruction_counter,instruction, instruction.opcode_mode)}

		match instruction.opcode_mode{
			OpcodeMode::Lit 			=> self.push_lit(instruction, instruction.address),
			OpcodeMode::Var =>{
				//	we need to make a copy of the var indicated by instruction' block and address
				let operand = self.operand_stack.fetch_local_var(instruction.block_num, instruction.address);
				self.push_copy(&operand, instruction);	
			}
			OpcodeMode::VarRef=>{
				let operand = self.operand_stack.fetch_local_var(instruction.block_num, instruction.address);
				if let CplDataType::CplVarRef(_) = operand.var{
					self.operand_stack.push(&operand);
				}else{
					self.push_copy(&operand, instruction);
				}
			}
			OpcodeMode::Arg				=> self.push_arg(instruction, self.instruction_counter),

			//	If we're sitting on top of an array, we can just add the element directly to it
			OpcodeMode::Array			=> self.operand_stack.push_array_element(),

			_=> abend!(format!("From exec_push: I don't know this address mode {}", instruction.opcode_mode)),
		}

		if self.cli.is_debug_bit(DUMP_OPERANDS){self.dump_operands("after exec_push");}
	}

	//	Removes the item at the top of the operand stack
	fn exec_pop(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_pop: {}", self.code_block_num, self.instruction_counter, instruction)}
		
		//	pop the stack but throw it away
		self.operand_stack.pop();
	}

	//	Print always uses whatever is at the top of the current frame.  If that var
	//	is  VarRef then print whatever it's pointing at.  NOTE:  we only support
	//	a single level of indirection.  If the thing that the VarRef is pointing at
	//	is, itself, a VarRef, the print instruction fails.
	fn exec_print(&mut self, instruction : MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_print: {}", self.code_block_num, self.instruction_counter, instruction)}

		let mut tos_ref = self.operand_stack.fetch_tos_ref();

		if let CplDataType::CplVarRef(ref vr) = tos_ref.var {
			tos_ref = self.operand_stack.fetch_ref(vr.frame_num, vr.block_num, vr.address);
		}

		match tos_ref.var{
				CplDataType::CplNumber(ref v) => {
					//	and print the value
					println!("{}", v.cpl_number);
				}

				CplDataType::CplString(ref v) => {
					//	and print the value
					println!("{}", v.cpl_string);
				}

				CplDataType::CplBool(ref b) => {
					//	and print the value
					println!("{}", b.cpl_bool);
				}

				CplDataType::CplArray(ref a) => {
					println!("{}",a);
				}

				_ => println!("Can't print: {}", tos_ref.var),
		}

		//	When we're done printing, consume the top of stack
		self.operand_stack.pop();
	}

	//	For all arithmatic, try to convert the operands to numbers, if they
	//	are strings.  panic if this isn't possible.
	fn pop_to_number (&mut self, instruction : &MachineInstruction) -> CplVar{
		let tos= self.operand_stack.dereference_tos();		
		match tos.var{
			CplDataType::CplNumber(_) => CplVar::new(tos.var.clone()),
			CplDataType::CplString(ref s) => {
				match s.cpl_string.parse(){
					Ok(n) => CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Real,n))),
					Err(_) => {
						abend!(format!("ERROR: operation \"{}\" can only be performed on numbers! Operand is \"{}\"", instruction.opcode, s.cpl_string));
						//areturn undefined();
					},
				}
			},
			_ => {
				abend! (&format!("Operation {} can't be performed on {}!", instruction.opcode,tos.var));
				//return undefined();
			},
		}
	}


	//  add the top two items of the stack and leave the result on the stack
	fn exec_add(&mut self, instruction : &MachineInstruction){
		let tos2 = self.pop_to_number(instruction);
		let tos1 = self.pop_to_number(instruction);

		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_add: {} ({}+{})", self.code_block_num, self.instruction_counter, instruction, tos1, tos2)}

		match tos1.var{
			CplDataType::CplNumber(ref v1) =>{
				match tos2.var{
					CplDataType::CplNumber(v2) => {
						let rslt = v1.cpl_number + v2.cpl_number;
						self.operand_stack.push(&CplVar::new(CplDataType::CplNumber(CplNumber::new(v1.rust_data_type,rslt))));
						if self.cli.is_debug_bit(DUMP_OPERANDS){self.dump_operands("at exec_add")}
						return;
					},
					_=>{},
				}
			},
			_=>{},
		}
		abend!(format!("From add:  can only add scalar values {}", tos1.var));
	}

	fn exec_sub(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_sub:  {}", self.code_block_num, self.instruction_counter, instruction)}

		let rslt = self.sub_help(instruction);

		self.operand_stack.push(&CplVar::new(CplDataType::CplNumber(CplNumber::new(rslt.0,rslt.1))));

		if self.cli.is_debug_bit(DUMP_OPERANDS){self.dump_operands("at exec_sub");}
	}

	//	This is used by both sub and comparison operands
	fn sub_help(&mut self, instruction : &MachineInstruction) -> (RustDataType, f64){

		let tos2 = self.pop_to_number(instruction);
		let tos1 = self.pop_to_number(instruction);

		match tos1.var{
			CplDataType::CplNumber(ref v1) => {
				match tos2.var{
					CplDataType::CplNumber(v2) => {
						return (v1.rust_data_type, v1.cpl_number - v2.cpl_number);
					},
					_=>{},
				}
			},
			_=>{}
		}
		abend!(format!("From sub_help:  can only operate on scalar values {}", tos1.var));
	}	

	fn exec_mul(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_mul: {}", self.code_block_num, self.instruction_counter, instruction)}

		let tos2 = self.pop_to_number(instruction);
		let tos1 = self.pop_to_number(instruction);

		match tos1.var{
			CplDataType::CplNumber(ref v1) => {
				match tos2.var{
					CplDataType::CplNumber(v2) => {
						let rslt = v1.cpl_number * v2.cpl_number;
						self.operand_stack.push(&CplVar::new(CplDataType::CplNumber(CplNumber::new(v1.rust_data_type,rslt))));
						if self.cli.is_debug_bit(DUMP_OPERANDS){self.dump_operands("at exec_mul")}
						return;
					},
					_=>{},
				}
			},
			_=>{}
		}
		abend!(format!("From mul:  can only multiply scalar values {}", tos1.var));
	}

	fn exec_div(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_div:  {}", self.code_block_num, self.instruction_counter, instruction)}

		let tos2 = self.pop_to_number(instruction);
		let tos1 = self.pop_to_number(instruction);

		match tos1.var{
			CplDataType::CplNumber(ref v1) => {
				match tos2.var{
					CplDataType::CplNumber(v2) => {
						let rslt = v1.cpl_number / v2.cpl_number;
						self.operand_stack.push(&CplVar::new(CplDataType::CplNumber(CplNumber::new(v1.rust_data_type,rslt))));
						if self.cli.is_debug_bit(DUMP_OPERANDS){self.dump_operands("at exec_div")}
						return;
					},
					_=>{},
				}
			},
			_=>{}
		}
		abend!(format!("From div:  can only divide scalar values {}", tos1.var));
	}


	fn exec_mod(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_mod:  {}", self.code_block_num, self.instruction_counter, instruction)}

		let tos2 = self.pop_to_number(instruction);
		let tos1 = self.pop_to_number(instruction);

		match tos1.var{
			CplDataType::CplNumber(ref v1) => {
				match tos2.var{
					CplDataType::CplNumber(v2) => {
						let rslt = v1.cpl_number % v2.cpl_number;
						self.operand_stack.push(&CplVar::new(CplDataType::CplNumber(CplNumber::new(v1.rust_data_type,rslt))));
						if self.cli.is_debug_bit(DUMP_OPERANDS){self.dump_operands("at exec_mod")}
						return;
					},
					_=>{},
				}
			},
			_=>{}
		}
		abend!(format!("From mul:  can only mod scalar values {}", tos1.var));
	}

	fn exec_bw_or(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_bw_or:  {}", self.code_block_num, self.instruction_counter, instruction)}

		let tos2 = self.pop_to_number(instruction);
		let tos1 = self.pop_to_number(instruction);

		match tos1.var{
			CplDataType::CplNumber(ref v1) => {
				match tos2.var{
					CplDataType::CplNumber(v2) => {
						let rslt = ((v1.cpl_number as i32) | (v2.cpl_number as i32)) as f64;
						self.operand_stack.push(&CplVar::new(CplDataType::CplNumber(CplNumber::new(v1.rust_data_type,rslt))));
						return;
					},
					_=>{},
				}
			},
			_=>{}
		}
		abend!(format!("From exec_bw_or:  can only Bitwise OR scalar values {}", tos1.var));
	}



	fn exec_bw_and(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_bw_and:  {}", self.code_block_num, self.instruction_counter, instruction)}

		let tos2 = self.pop_to_number(instruction);
		let tos1 = self.pop_to_number(instruction);

		match tos1.var{
			CplDataType::CplNumber(ref v1) => {
				match tos2.var{
					CplDataType::CplNumber(v2) => {
						let rslt = ((v1.cpl_number as i32) & (v2.cpl_number as i32)) as f64;
						self.operand_stack.push(&CplVar::new(CplDataType::CplNumber(CplNumber::new(v1.rust_data_type,rslt))));
						return;
					},
					_=>{},
				}
			},
			_=>{}
		}
		abend!(format!("From mul:  can only Bitwise AND scalar values {}", tos1.var));
	}

	fn exec_concat(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_concat: {}", self.code_block_num, self.instruction_counter, instruction)}

		let tos2 = self.operand_stack.dereference_tos();
		let tos1 = self.operand_stack.dereference_tos();

		let tos2_string : String;

		//	Test the concatinate.  If it's a String then leave it alone.  If it's
		//	a scalar, then stringify it.
		match tos2.var{
			CplDataType::CplNumber(v) => tos2_string = v.cpl_number.to_string(),
			CplDataType::CplString(v) => tos2_string = v.cpl_string,
			CplDataType::CplBool(v) => tos2_string = v.cpl_bool.to_string(),
			_ => abend!(format!("from concat:  data to append must be either a scalar or a string. it is {}", tos2.var)),
		}				

		//  If tos - 1 is a scalar (number) then stringify it
		//	If tos - 1 is already a string, then do nothing to it
		match tos1.var{
			CplDataType::CplString(ref v1) => self.do_concat_and_push(&v1.cpl_string, &tos2_string),
			CplDataType::CplNumber(ref v1) => self.do_concat_and_push(&v1.cpl_number.to_string(), &tos2_string),
			CplDataType::CplBool(ref v1) => self.do_concat_and_push(&v1.cpl_bool.to_string(), &tos2_string),
			_=> abend!(format!("From concat:  I don't understand this {}", tos1.var)),
		}

		if self.cli.is_debug_bit(DUMP_OPERANDS){self.dump_operands("at exec_concat");}
	}

	//	for hackableness
	fn do_concat_and_push(&mut self, var : &str, append : &str){
		let mut rslt : String = var.to_string();
		rslt.push_str(append);

		//	get the current operand_frame
		self.operand_stack.push(&CplVar::new(CplDataType::CplString(CplString::new(rslt))));
	}	


	//	This updates a local operand in situ via the address in the instruction
	fn update_scalar(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("      update_scalar(start): {}", instruction)}

		self.operand_stack.update_local(instruction.block_num, instruction.address);

		if self.cli.is_debug_bit(TRACE_EXEC){println!("      update_scalar(end): {} {}", instruction, self.operand_stack.fetch_local_var(instruction.block_num, instruction.address))}
	}

	//	if the mode is UpdateIndexed and the target is an actual colletion
	//	update it directly
	fn update_indexed_direct(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("      update_indexed_direct: {} {}", instruction, self.operand_stack.fetch_local_var(instruction.block_num, instruction.address))}
		self.operand_stack.update_local_collection(instruction.block_num, instruction.address);
	}

	//	As noted, this is a bit tricky:  we need to get a rust reference to the
	//	operand pointed to by the VarRef
	fn update_indexed_indirect(&mut self, instruction : &MachineInstruction, current_frame_num : usize){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("      update_indexed_indirect: {} {}", instruction, self.operand_stack.fetch_local_var(instruction.block_num, instruction.address))}
	
		let value = self.operand_stack.dereference_tos();
		let index = self.operand_stack.dereference_tos();

		let collection_ref = &self.operand_stack.operand_frames[current_frame_num].operand_blocks[instruction.block_num].operand_block[instruction.address];

		if let CplDataType::CplVarRef(ref vr) = &collection_ref.var{

			//	So here is an example of getting around the borrow checker.  You'd like to
			//	think that you could use te fields of vr (var ref) directly in the next
			//	statement below ("let collection = {...}").  But NO!.  If you substitute
			//	vr.frame_num for coll_frame in that statement you get:
			//
			//	cannot borrow `self.operand_stack.operand_frames` as mutable because it is also borrowed as immutable
			//
			//	so, the solution, evidently, is to create local variables to hold these fields.
			let coll_frame = vr.frame_num;
			let coll_block = vr.block_num;
			let coll_address = vr.address;

			let collection = &mut self.operand_stack.operand_frames[coll_frame].operand_blocks[coll_block].operand_block.get_mut (coll_address).unwrap();

			match &mut collection.var{
				CplDataType::CplArray(a) => if instruction.opcode == Opcode::Update{
					a.update_indexed(&index, &value);
				}else{
					a.update_indexed_op(&index, &value, instruction.opcode);
				},
				CplDataType::CplDict(d) => if instruction.opcode == Opcode::Update{
					d.update_indexed(&index, &value);
				}else{
					d.update_indexed_op(&index, &value, instruction.opcode);
				},
				_ => panic!("from update_indexed_indirect:  expected an array or dictionary, got {}",collection.var),
			}
		}

	}


	//	If the mode is UpdateIndexed, the address in the instruction is the
	//	address of an array or a VarRef.  TOS is the new value and TOS-1 is the index.
	//  We want to update "in situ" because this is an lvalue.
	//
	//	This is where it gets tricky:  if the opcode mode is UpdateIndexed we have
	//	two possibilities:
	//
	//		1)	the target of the update is an actual collection (e.g. array)
	//		2)	the target of the update is a VarRef in which case what's being
	//			updated is pointed to by the VarRef.
	//
	//	If the target is a VarRef we don't want to simply dereference because it could be
	//	an array with 500,000 element in it.
	fn update_indexed (&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("      update_indexed: {} {}", instruction, self.operand_stack.fetch_local_var(instruction.block_num, instruction.address))}

		let frame_num = self.operand_stack.operand_frames.len()-1;

		match self.operand_stack.operand_frames[frame_num].operand_blocks[instruction.block_num].operand_block[instruction.address].var{
			CplDataType::CplVarRef(_) => self.update_indexed_indirect(&instruction, frame_num),
			_ => self.update_indexed_direct(&instruction),
		}
	}

	//	Adds a Var to an array that is at the top of the stack.  Tos is the value to update.
	//	Tos-1 is the array to update.
	fn update_collection_array(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("      update_collection_array: {} {} {}", self.code_block_num, self.instruction_counter, instruction)}
		self.operand_stack.push_array_element();
	}

	//	adds a new key/value pair to a dictionary.  The key/value pair are at tos and tos-1.
	//	The dictionary is at tos-2
	fn update_collection_dict(&mut self){
		self.operand_stack.insert_dict();
	}

	//	If the mode is Update, then update the value at the address specified in the instruction
	//	if the mode is Array then update the the array at tos-1 with the new value at tos.
	//	if the mode is Dict then update the dictionary at tos-2  with the key/value pair at
	//	tos and tos -1.
	//  If the mode is UpdateIndexed, see comments at "update_indexed"
	fn exec_update(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_update: {}", self.code_block_num, self.instruction_counter, instruction)}

		//self.dump_operands("begin exec_update");

		match instruction.opcode_mode{
			OpcodeMode::Update => self.update_scalar(instruction),
			OpcodeMode::Array => self.update_collection_array(instruction),
			OpcodeMode::Dict => self.update_collection_dict(),
			OpcodeMode::UpdateIndexed => self.update_indexed(instruction),
			_=> abend!(format!("From exec_update:  I don't know what this means: {}", instruction.opcode_mode)),
		}

		//self.dump_operands("end exec_update");

		if self.cli.is_debug_bit(DUMP_OPERANDS){self.dump_operands("at exec_update")}
	}

	/**********************************************************************
	***		Assignment Operators (e.g. +=, -=, etc.)
	**********************************************************************/

	//  Apply an operator to an lvalue.  If the lvalue is an indexed value (i.e. if
	//	the mode is "UpdateIndexed") then the target is a collection.  Otherwise it's a
	//	scalar.
	fn exec_assignment_operator(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_assignment_operator: {}", self.code_block_num, self.instruction_counter, instruction)}

		if instruction.opcode_mode == OpcodeMode::UpdateIndexed{
			//	if the mode is indexed then update an element of an array
			self.apply_binary_operator_indexed(instruction);
		}else{
			//	Otherwise we just updating a scalar.  E.G.  a += foo;

			match self.operand_stack.fetch_local_var(instruction.block_num, instruction.address).var{

				CplDataType::CplNumber(_) | CplDataType::CplString(_) => {
					//	If the collection is direct (i.e. an array or dictionay) do this
					// n.apply_binary_operator_to_number(&new_value_var, instruction.opcode);
					self.operand_stack.apply_binary_operator_scalar(instruction.block_num, instruction.address, instruction.opcode);
				}	
	
				//	otherwise, get a mutable reference to the collection and then, depending on the type
				//	of collection update the element at the index
				CplDataType::CplVarRef(vr) =>{
					let new_value_var = self.operand_stack.dereference_tos();
					let varref : &mut CplVar = self.operand_stack.fetch_mutable_ref(vr.frame_num, vr.block_num, vr.address);
					match varref.var{
						CplDataType::CplNumber (ref mut n) => {
							n.apply_binary_operator_to_number(&new_value_var, instruction.opcode);
						}

						CplDataType::CplString (ref mut s) =>{
							s.apply_binary_operator_to_string(&new_value_var, instruction.opcode);
						}
						_=> panic!("from exec.apply_binary_operator_scalar:  expected a VarRef to array or dictionary.  Got {}",varref.var),
					}
				}
	
				_=> panic!("from exec.apply_binary_operator_scalar:  unknown type: {}", self.operand_stack.fetch_local_var(instruction.block_num, instruction.address).var),
			}
		}
	}

	
	//	Apply and operator to an element of a collection
	fn apply_binary_operator_indexed(&mut self, instruction : &MachineInstruction){
		let var = self.operand_stack.fetch_local_var(instruction.block_num, instruction.address);
	
		let new_value = self.operand_stack.dereference_tos();
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

	fn exec_unary_op(&mut self, instruction : &MachineInstruction){
		self.operand_stack.perform_unary_op(instruction.address, instruction.opcode);
	}

	//	A special purpose opcode used internally for diagnosit purposes
	fn exec_diag(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("**** {}", instruction.literal.token_value)}
		//println!("***** {}",instruction.literal.token_value);
	}

	//	hackability helper.  Get the argument at the top of the stack, clear it and return it.
	fn grab_an_argument(&mut self, _opcode_mode : OpcodeMode) -> CplVar{

		let tos = self.operand_stack.pop();
		match tos.var {
			CplDataType::CplNumber(ref v) => {
				// println!("================ grab_an_argument match number {}, mode={}",v,_opcode_mode);
				return CplVar::new(CplDataType::CplNumber(CplNumber::new(v.rust_data_type, v.cpl_number)))
			},
			CplDataType::CplBool(b) => return CplVar::new(CplDataType::CplBool(CplBool::new(b.cpl_bool))),
			CplDataType::CplString(v) => return CplVar::new(CplDataType::CplString(CplString::new(v.cpl_string.clone()))),
			CplDataType::CplVarRef(v) => {
				// println!("================== grab_an_argument match VarRef {}",v);
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
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_function_call: {}", self.code_block_num, self.instruction_counter, instruction)}

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

		//	Now, if the called function is external, call it directly (somehow),
		//	otherwise we launch a new executor
		if instruction.opcode_mode == OpcodeMode::Extern{
			let rslt = (self.builtin_functions.builtin_function_list.get_mut(instruction.block_num).unwrap().target)(&mut self.builtin_functions, &arguments, &mut self.operand_stack);
			if self.cli.is_debug_bit(TRACE_EXEC){println!("      return from external \"{}\" rslt={}", instruction.literal.token_value, rslt)}
			self.operand_stack.push(&rslt);
			return;
		}


		let code_frame_num = instruction.block_num;

		let arguments_len = arguments.len();

		//	instantiate a new exec object
		let mut executor = Executor::call(self.cli, self.code_frames, code_frame_num, &mut arguments, &mut self.operand_stack, self.builtin_functions, arguments_len);

		//	This is the actual call to the run function
		let return_value = executor.exec();

		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : Return from function call: {} return value {}", self.code_block_num, self.instruction_counter, instruction, return_value)}

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
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_alloc: {}", self.code_block_num, self.instruction_counter, instruction)}
		if self.cli.is_debug_bit(DUMP_OPERANDS){self.dump_operands("at exec_alloc");}

		//let block_num = self.operand_stack.current_block_num();

		self.operand_stack.alloc(instruction.block_num, instruction.address);
	}

	/******************************************************************
	*** Jump Instructions
	******************************************************************/
	fn exec_j (&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_j: {}", self.code_block_num, self.instruction_counter, instruction)}

		// if instruction.address >= self.code_frames[self.code_frame_num].code_frame.len() as i32{
		// 	panic! ("From exec_j: address out of bounds {}", instruction.address);
		// }
		self.instruction_counter = instruction.address;
	}

	fn exec_jt(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_jt: {}", self.code_block_num, self.instruction_counter, instruction)}

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
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_jf: {}", self.code_block_num, self.instruction_counter, instruction)}

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

	//	Branch and Link is a lightweight subroutine call:
	//
	//		push the return address onto a stack.  The return address is
	//		the instruction frame number and the instruction address	
	//		transfer control to the address 0 of a new block.  The new
	//		block is always the return block + 1
	fn exec_bl(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_bl: {}", self.code_block_num, self.instruction_counter, instruction)}
		
		//	This is where the BlockEnd instruction jumps to
		self.block_end_return_address.push((instruction.block_num, instruction.address));

		//  set the new block number from the qualifier in the instruction
		self.code_block_num = instruction.qualifier[0];

		//	start at the first instruction (the instruction counter is incremented
		//	after each instruction so we start at -1 so the next instruction will
		//	be at 0)
		self.instruction_counter = 0;
	}



	fn exec_break (&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_break: {}", self.code_block_num, self.instruction_counter, instruction)}
		let mut n : i32 = self.block_end_return_address.len() as i32;
		while n >= 0{
			self.block_end_return_address.pop();
			n -= 1;
		}

		self.code_block_num = instruction.block_num;
		self.instruction_counter = instruction.address;

		self.block_counter -= 1;
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{} : exec_break:  return to {} block {}", self.instruction_counter, self.code_block_num, self.block_counter);}

		//	pop the operand stack block
		self.operand_stack.pop_block();			
		
	}

	fn exec_continue (&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_continue: {}", self.code_block_num, self.instruction_counter, instruction)}

		while !self.block_end_return_address.is_empty(){
			let _temp = self.block_end_return_address.pop().unwrap();
		}

		self.code_block_num = instruction.block_num;
		self.instruction_counter = instruction.address;

		self.block_counter -= 1;

		//  ****************** revisit this.... I think we're removing blocks and not variables

		let mut pop_count = self.operand_stack.variable_count();
		while pop_count > 1{
			//	pop the operand stack frame
			self.operand_stack.pop();
			pop_count -= 1;
		}
	}

	/******************************************************************
	*** Comparison Operators
	******************************************************************/
	//	These compare the top two itme on the stack by logically
	//	subtracting TOS-1 from TOS.  The result is analyzed:
	//
	//		< 0, lt and ne are true
	//		0, eq, ge and le are all true
	//		> 0, gt and ne are true
	//
	//	And either "true" or "false" is pushed on to the stack.
	//  For example if the infix expression was 10 > 20, the postfix expression would be 10 20 >
	//	and subtracing tos-1 from tos is 10 - 20 which is a negative number so, lt and ne are true
	//	and specifically, gt is false.
	//
	//	In other words, what is left on the operand stack is the boolean resuslt of
	//	the comparison

	fn exec_lt(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_lt: {}", self.code_block_num, self.instruction_counter, instruction)}

		let rslt = self.sub_help(instruction);

		if rslt.1 < 0.0 {
			self.push_lit_bool_help(true, self.instruction_counter, instruction);
		}else{
			self.push_lit_bool_help(false, self.instruction_counter, instruction);
		}
	}
	fn exec_gt(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_gt: {}", self.code_block_num, self.instruction_counter, instruction)}

		let rslt = self.sub_help(instruction);

		if rslt.1 > 0.0 {
			self.push_lit_bool_help(true, self.instruction_counter, instruction);
		}else{
			self.push_lit_bool_help(false, self.instruction_counter, instruction);
		}
	}
	fn exec_le(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_le:  {}", self.code_block_num, self.instruction_counter, instruction)}

		let rslt = self.sub_help(instruction);

		if rslt.1 <= 0.0 {
			self.push_lit_bool_help(true, self.instruction_counter, instruction);
		}else{
			self.push_lit_bool_help(false, self.instruction_counter, instruction);
		}

	}
	fn exec_ge(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_ge: {}", self.code_block_num, self.instruction_counter, instruction)}

		let rslt = self.sub_help(instruction);

		if rslt.1 >= 0.0 {
			self.push_lit_bool_help(true, self.instruction_counter, instruction);
		}else{
			self.push_lit_bool_help(false, self.instruction_counter, instruction);
		}
	}

	//	Compare a number to a string that might also be a number.  If the string
	//	can't be converted to a number then return false else return the
	//	result of the comparison.  If the warnings switch was set, print a warning
	//	if the string could not be converted to a number
	fn eq_ne_help1(&self, vnumber : f64, vstring : &str, eq : bool) -> bool{
		match vstring.parse(){
			Ok(n) => if eq { return vnumber == n} else {return vnumber != n}
			Err(_) => {
				if self.cli.is_runtime_warnings() {
					if eq{
						//	if this is helping an eq print this one as a warning
						println!("Warning: unable to compare {} == {} returning false", vnumber, vstring);
					}else{
						//	if this is helping a ne then print this warning message
						println!("Warning: unable to compare {} != {} returning false", vnumber, vstring);
					}
				}
				return false;
			}
		}
	}

	//	compare the types of two variables:  return true if they are the
	//	the same else false
	fn compare_types(&self, var1 : CplVar, var2 : CplVar) -> bool{
		var1.is_type_equal(&var2)
	}

	//	performs either eq or ne on two vars.  "eq" true means compare equal
	//	"eq" false means compare not equal
	fn eq_ne_help(&self, var1:CplVar, var2:CplVar, eq : bool) -> bool{
		//  first compare the types.  If the two types cannot be compared (e.g.
		//	one of them is an array) then return false.	
		if self.compare_types(var1.clone(), var2.clone()){
			if eq {
				return var1.is_equal(&var2,self.cli.is_runtime_warnings());
			}else{
				return var1.is_not_equal(&var2,self.cli.is_runtime_warnings());
			}
		}

		//	Now try comparing the actual values (i.e. the two operands are scalar value)
		if let CplDataType::CplNumber(ref n1) = var1.var{
			if let CplDataType::CplNumber(ref n2) = var2.var{
				if eq {
					return n1.cpl_number == n2.cpl_number;
				}else{
					return n1.cpl_number != n2.cpl_number;
				}
			}else if let CplDataType::CplString(ref s2) = var2.var{
				return self.eq_ne_help1(n1.cpl_number, &s2.cpl_string, eq);
			}else{
				if self.cli.is_runtime_warnings(){
					println!("Warning: Unable to compare {} with {} returning false", n1.cpl_number, var2.var);
				}
				return false;
			}
		} else if let CplDataType::CplString(ref s1) = var1.var{
			if let CplDataType::CplString(ref s2) = var2.var{
				if eq {
					return s1.cpl_string == s2.cpl_string;
				}else{
					return s1.cpl_string != s2.cpl_string;
				}
			}else if let CplDataType::CplNumber(ref n2) = var2.var{
				return self.eq_ne_help1(n2.cpl_number, &s1.cpl_string, eq);
			}else{
				if self.cli.is_runtime_warnings(){
					println!("Warning: Unable to compare {} with {} returning false", s1.cpl_string, var2.var);
				}
				return false;
			}
		} else {
			if self.cli.is_runtime_warnings(){
				println!("Warning: Unable to compare {} with {} returning false", var1.var, var2.var);
			}
			return false;
		}
	}

	fn exec_ne(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_ne: {}", self.code_block_num, self.instruction_counter, instruction)}

		let tos2 = self.operand_stack.dereference_tos();
		let tos1 = self.operand_stack.dereference_tos();

		let rslt = self.eq_ne_help(tos1, tos2, false);

		if rslt {
			self.push_lit_bool_help(true, self.instruction_counter, instruction);
		}else{
			self.push_lit_bool_help(false, self.instruction_counter, instruction);
		}
	}

	//	compare equal top two elements 
	fn exec_eq(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_eq: {}", self.code_block_num, self.instruction_counter, instruction)}

		let tos2 = self.operand_stack.dereference_tos();
		let tos1 = self.operand_stack.dereference_tos();
		
		let rslt = self.eq_ne_help(tos1, tos2, true);
		if rslt {
			self.push_lit_bool_help(true, self.instruction_counter, instruction);
		}else{
			self.push_lit_bool_help(false, self.instruction_counter, instruction);
		}
	}

	//	Perform OR or AND on two variables, return true if result is true.
	//	if lor is true then do OR, else do AND
	fn lor_land_help(&self, var1:CplVar, var2:CplVar, lor : bool) -> bool{
		if let CplDataType::CplBool(ref b1) = var1.var{
			if let CplDataType::CplBool(ref b2) = var2.var{
				if lor {
					return b1.cpl_bool || b2.cpl_bool;
				}else{
					return b1.cpl_bool && b2.cpl_bool;
				}
			}
		}

		panic!("from lor_land_help: expected two boolean values.  Got {} and {}", var1, var2);
	}

	fn exec_lor(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_lor:  {}", self.code_block_num, self.instruction_counter, instruction)}

		let tos2 = self.operand_stack.dereference_tos();
		let tos1 = self.operand_stack.dereference_tos();
		
		let rslt = self.lor_land_help(tos1, tos2, true);
		if rslt {
			self.push_lit_bool_help(true, self.instruction_counter, instruction);
		}else{
			self.push_lit_bool_help(false, self.instruction_counter, instruction);
		}
	}


	fn exec_land(&mut self, instruction : &MachineInstruction){
		if self.cli.is_debug_bit(TRACE_EXEC){println!("{}:{} : exec_land:  {}", self.code_block_num, self.instruction_counter, instruction)}

		let tos2 = self.operand_stack.dereference_tos();
		let tos1 = self.operand_stack.dereference_tos();
		
		let rslt = self.lor_land_help(tos1, tos2, false);
		if rslt {
			self.push_lit_bool_help(true, self.instruction_counter, instruction);
		}else{
			self.push_lit_bool_help(false, self.instruction_counter, instruction);
		}
	}


}