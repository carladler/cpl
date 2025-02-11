//use std::fmt;
use std::collections::HashMap;
use builtin::*;
use machineinstruction::*;
use macrolib::*;


/****************************************
****	Frame Map
*****************************************/

//  The frame map links a name to the code space for a function.
#[derive(Clone)]
pub struct FrameMap{
	//cli : & 'a CLI,	
	//	map a name to an index into the list.  The name maps to
	//	the index AND a flag indicating whether or not it's a
	//	builtin function
	pub frame_names : HashMap<String, (usize, bool)>,
	//	a list of frames
	pub frames_list : Vec<CodeFrame>,
	pub builtin_function_table : BuiltinFunctions,
}

impl FrameMap{
	pub fn new() -> FrameMap{
		let mut fm = FrameMap{
			//cli : cli,
			frame_names : HashMap::new(),
			frames_list : Vec::new(),
			builtin_function_table : BuiltinFunctions::new(),
		};


		let mut builtin_function_num = 0;
		for builtin_function in &fm.builtin_function_table.builtin_function_list{
			fm.frame_names.insert(builtin_function.name.clone(), (builtin_function_num, true) );
			fm.frames_list.push(CodeFrame::new(&builtin_function.name, false, true, builtin_function.arguments.clone()));
			builtin_function_num += 1;
		}
		fm
	}

	//	Adds a new Frame to the Frame Map.  The Hash table provides a function
	//	name lookup that maps to the actual frame (opcode store) via its index
	pub fn add_new_frame(&mut self, name : String, code_frame : CodeFrame){
		self.frames_list.push(code_frame);
		self.frame_names.insert(name, (self.frames_list.len() - 1, false));
	}

	pub fn code_dump(&self){
		//  do this unconditionally:  the debug bit is checked by the callers
		for f in &self.frames_list{
			if f.is_builtin{
				println!("{} is Builtin", f.function_name);
			}else{
				println!("Generated Code for Function {}", f.function_name);
				f.code_dump();
			}
		}		
	}

	pub fn get_entry_frame_number(&self) -> usize{
		let mut i = 0;
		for f in &self.frames_list{
			if f.is_entry {return i as usize}
			i += 1;
		}

		abend!(format!("from get_entry_frame_number.  Couldn't locate the entry frame"));
	}

}
/****************************************
****	Code Block
*****************************************/
//	A Code Block holds all of the instructions generated as a result of
//	a function or statement that has a block (e.g. IF, WHILE, etc).
#[derive(Clone)]
pub struct CodeBlock{
	//cli : & 'a CLI,
	//pub block_kind : BlockKind,
	pub breakable : bool,
	pub code_block : Vec<MachineInstruction>,
	pub address_counter : usize,		// where the next instruction goes
	//branch_and_link : Vec<(usize,usize)>,	// branch and link:  return to (block_num,address)
	//fix_jump_addresses : Vec<usize>,
}

impl CodeBlock {
//	fn new (cli : & 'a CLI, block_kind : BlockKind) -> CodeBlock{
//	fn new (cli : & 'a CLI, breakable : bool) -> CodeBlock{
	pub fn new (breakable : bool) -> CodeBlock{
			CodeBlock{
			//cli : cli,
			//block_kind : block_kind,
			breakable : breakable,
			code_block : Vec::new(),
			address_counter : 0,
			//branch_and_link : Vec::new(),
			//fix_jump_addresses : Vec::new(),
		}
	}

	fn code_dump(&self){
		let mut addr = 0;
		for instruction in &self.code_block{
			println!("    {}: {}",addr, instruction);
			addr += 1;
		}
	}

	pub fn get_current_address (&self) -> usize{
		self.address_counter
	}

	pub fn get_address_counter(&self) -> usize{
		self.address_counter
	}


	//	Add an opcode to this block
	fn add_machine_instruction(&mut self, instruction : MachineInstruction){
		// println!(".... add_machine_instruction: Adding Instruction: {} : {}", self.address_counter, instruction);

		self.code_block.push(instruction);
		self.address_counter += 1;
	}	
}


/****************************************
****	Code Frame
*****************************************/

//	A code frame contains all of the code for a function.  The code is partitioned into
//	Code Blocks.
#[derive(Clone)]
pub struct CodeFrame{
	//cli : & 'a CLI,
	pub function_name : String,
	pub is_entry : bool,				// if true then this frame holds the entry function
	pub is_builtin : bool,
	pub parameters : Vec<String>,					// parameters for the function
	pub code_block_list : Vec<CodeBlock>,			// The list intermediate code blocks
	pub current_code_block_num : Vec<usize>,		// Which code block are we adding instructions to
}

impl CodeFrame {
	pub fn new(function_name : &str, is_entry : bool, is_builtin : bool, parameters : Vec<String>) -> CodeFrame{
		CodeFrame{
			//cli : cli,
			function_name : function_name.to_string(),
			is_entry : is_entry,
			is_builtin : is_builtin,
			parameters : parameters,
			code_block_list : Vec::new(),
			current_code_block_num : Vec::new(),
		}
	}

	pub fn get_address_counter(&self) -> usize{
		let current_code_block_num : usize;
		match self.current_code_block_num.last(){
			Some(x) => current_code_block_num = *x,
			None    => current_code_block_num = 0,
		}

		self.code_block_list[current_code_block_num].get_address_counter()
	}

	//	Add an opcode to the current code block of the current frame
	pub fn add_machine_instruction(&mut self, instruction : MachineInstruction){
		//	Add the instruction to the current_code_block_num (the block at the top of the stack)
		let current_code_block_num : usize;
		match self.current_code_block_num.last(){
			Some(x) => current_code_block_num = *x,

			//	If the current_code_block_num stack is empty, this means that
			//	we're adding synthetic stuff, presumably at the end of a
			//	function
			None    => current_code_block_num = 0,
		}
		self.code_block_list.get_mut(current_code_block_num).unwrap().add_machine_instruction(instruction);
	}

	pub fn get_current_address(&self) -> usize{
		self.code_block_list[*self.current_code_block_num.last().unwrap()].get_current_address()
	}

	pub fn get_current_block_num(&self) -> usize{
		*self.current_code_block_num.last().unwrap()
	}


	pub fn code_dump(&self){
		let mut block_num = 0;
		for block in &self.code_block_list{
			println!("Block Number: {}", block_num);
			block.code_dump();
			block_num += 1;
		}
	}
}
