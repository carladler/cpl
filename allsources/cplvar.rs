#![allow(unused_imports)]
use std::fs::File;
use std::fs::metadata;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;
use std::io::BufRead;
use std::io::Write;
use std::cmp::Ordering;

use std::fmt;
use std::collections::HashMap;
use std::str;
use regex::Regex;

use opcode::*;
use tokenizer::*;
use macrolib::*;
use csvparser::*;

use std::cell::RefCell;
use names::*;


#[derive(PartialOrd, Ord, PartialEq, Copy, Clone, Eq, Hash)]
pub enum RustDataType{
	Uninitialized,
	Int,
	Real,
	String,
	Bool,
	Char,
	Opcode,
	FileHandle,
	NONE,
}

impl fmt::Display for RustDataType{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
       match self {
			RustDataType::Uninitialized => write!(f, "Uninitialized"),
			RustDataType::Int => write!(f, "int"),
			RustDataType::Real => write!(f, "real"),
			RustDataType::String => write!(f, "string"),
			RustDataType::Bool => write!(f, "bool"),
			RustDataType::Char => write!(f, "char"),
			RustDataType::Opcode => write!(f, "opcode"),
			RustDataType::FileHandle => write!(f, "filehandle"),
			RustDataType::NONE => write!(f, "NONE"),
	   }
	}
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum CplDataType{
	CplNumber(CplNumber),
	CplString(CplString),
	CplBool(CplBool),
	CplArray(CplArray),
	CplStruct(CplArray),					// An array with a different type name
	CplDict(CplDict),
	CplVarRef(CplVarRef),					// pointer to a CplVar
	CplFileReader(CplFileReader),
	CplFileWriter(CplFileWriter),
	CplFileAppender(CplFileAppender),
	CplUninitialized(CplUninitialized),
	CplUndefined(CplUndefined),
}
impl fmt::Display for CplDataType{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    	match self {
			CplDataType::CplUninitialized(_) => write!(f,"CplUninitialized"),
			CplDataType::CplUndefined(_) => write!(f, "CplUndefined"),
			CplDataType::CplNumber(s) => write!(f,"CplNumber({})", s.cpl_number),
			CplDataType::CplBool(b) => write!(f,"CplBool: {}", b.cpl_bool),
			CplDataType::CplVarRef(r) => write!(f,"CplVarRef: {},{},{}",r.frame_num, r.block_num, r.address),
			CplDataType::CplArray(a) => write!(f,"CplArray: {}",a.array_text()),
			CplDataType::CplDict(_) => write!(f,"CplDict"),
			CplDataType::CplString(s) => write!(f,"\"{}\"", s.cpl_string),
			CplDataType::CplFileReader(s) => write!(f,"CplFileReader: \"{}\"", s.file_name),
			CplDataType::CplFileWriter(s) => write!(f,"CplFileWriter: \"{}\"", s.file_name),
			CplDataType::CplFileAppender(s) => write!(f,"CplFileAppender: \"{}\"", s.file_name),
			CplDataType::CplStruct(_) => write!(f,"CplStruct"),
	   }
	}
}

impl Clone for CplDataType{
	fn clone(&self) -> CplDataType{
    	match self {
			CplDataType::CplUninitialized(_) => CplDataType::CplUninitialized(CplUninitialized::new()),
			CplDataType::CplUndefined(_) => CplDataType::CplUndefined(CplUndefined::new()),
			CplDataType::CplNumber(v) => CplDataType::CplNumber(CplNumber::new(v.rust_data_type, v.cpl_number)),
			CplDataType::CplBool(b) => CplDataType::CplBool(CplBool::new(b.cpl_bool)),
			CplDataType::CplVarRef(r) => CplDataType::CplVarRef(CplVarRef::new(r.frame_num, r.block_num, r.address)),
			CplDataType::CplArray(a) => CplDataType::CplArray(a.clone()),
			CplDataType::CplDict(d) => CplDataType::CplDict(d.clone()),
			CplDataType::CplString(v) => CplDataType::CplString(CplString::new(v.cpl_string.clone())),
			CplDataType::CplFileReader(_) => abend!(format!("Unable to clone CplFileReader")),
			CplDataType::CplFileWriter(_) => abend!(format!("Unable to clone CplFileWriter")),
			CplDataType::CplFileAppender(_) => abend!(format!("Unable to clone CplFileAppender")),
			CplDataType::CplStruct(a) => CplDataType::CplStruct(a.clone()),
	   }
	}
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum CplDataTypeInspected{
	CplNumber,
	CplString,
	CplBool,
	CplArray,
	CplStruct,
	CplDict,
	CplVarRef,
	CplUninitialized,
	CplUndefined,
	DontCare,
}
impl fmt::Display for CplDataTypeInspected{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    	match self {
			CplDataTypeInspected::CplUninitialized => write!(f,"CplUninitialized"),
			CplDataTypeInspected::CplUndefined => write!(f, "CplUndefined"),
			CplDataTypeInspected::CplNumber => write!(f,"CplNumber"),
			CplDataTypeInspected::CplBool => write!(f,"CplBool"),
			CplDataTypeInspected::CplVarRef => write!(f,"CplVarRef"),
			CplDataTypeInspected::CplArray => write!(f,"CplArray"),
			CplDataTypeInspected::CplDict => write!(f,"CplDict"),
			CplDataTypeInspected::CplString => write!(f,"CplString"),
			CplDataTypeInspected::CplStruct => write!(f,"CplStruct"),
			CplDataTypeInspected::DontCare => write!(f,"DontCare"),
	   }
	}
}


/****************************************
****	Operand Stack
*****************************************/
//	This is the CPU's "main" memory.  It is partitioned dynamically
//	into frames, which are created when a new executor is created and blocks
//	which are created for the Function, WHILE, IF, ELSE, etc.  In otherwords,
//
//		operand_stack
//			frame 0
//				block 0
//				block 1
//			frame 1
//				block 0
//				block 1
//			etc.
//	
//	That is, there is always at least 1 frame. Variables are created dynamically
//	within a function and placed on the operand stack in the frame belonging to
//	that function. A VarRef variable can refer to another variable in either the same
//	frame or a different frame.
//
//	The operand stack is created once by main() and a reference to it is passed
//	to all subsequent executors.  In other words, the operand stack is global to
//	an executing program.
pub struct OperandStack{
	pub operand_frames : Vec<OperandFrame>,
	cli_warnings : bool,
	namelist : Vec<String>,
}
impl OperandStack{
	pub fn new(cli_warnings : bool, namelist : Vec<String>) -> OperandStack{
		OperandStack{
			//  create the first frame when the operand stack is instantiated
			operand_frames : Vec::new(),
			cli_warnings : cli_warnings,
			namelist : namelist,
		}
	}


	pub fn current_frame(&self) -> usize{
		self.operand_frames.len() - 1
	}

	//	push a copy of a variable from a fully qualified address
	pub fn push_copy(mut self, frame_num:usize, block_num:usize, address:usize){
		let var = &self.fetch_var(frame_num, block_num, address);
		self.push(var);
	}

	//	push a local copy of the variable at the address onto the operand stack.
	//	local, as in the current block
	pub fn push_local_copy(&mut self, block_num : usize, address:usize){
		let var = &self.fetch_local_var(block_num, address);
		self.push(var);
	}

	//	push a variable onto the current block in the current frame
	pub fn push(&mut self, var : &CplVar){
		let operand_frame : &mut OperandFrame = self.operand_frames.last_mut().unwrap();
		let operand_block : &mut OperandBlock = operand_frame.operand_blocks.last_mut().unwrap();
		operand_block.operand_block.push(var.clone());
	}
	
	//	pop a variable off the current block in the current frame
	pub fn pop(&mut self) -> CplVar{
		let operand_frame = self.operand_frames.last_mut().unwrap();
		let operand_block = operand_frame.operand_blocks.last_mut().unwrap();
		operand_block.operand_block.pop().unwrap()
	}

	//	Returns the value at the top of the stack without destroying it
	pub fn peek(&mut self) -> CplVar{
		let operand_frame = self.operand_frames.last_mut().unwrap();
		let operand_block = operand_frame.operand_blocks.last_mut().unwrap();
		operand_block.operand_block.last_mut().unwrap().clone()
	}

	//	adds a new frame to the operand stack
	pub fn push_frame(&mut self){
		self.operand_frames.push(OperandFrame::new());
	}

	//	removes the last frame in the operand stack
	pub fn pop_frame(&mut self){
		self.operand_frames.pop().unwrap();
	}

	//	push a new block on the current frame
	pub fn push_block(&mut self){
		//	get the current frame
		let frame_num = self.operand_frames.len() - 1;
		let frame = self.operand_frames.get_mut(frame_num).unwrap();
		frame.operand_blocks.push(OperandBlock::new());
	}

	//	remove the last block of the current frame
	pub fn pop_block(&mut self){
		let frame_num = self.operand_frames.len() - 1;
		let frame = self.operand_frames.get_mut(frame_num).unwrap();
		frame.operand_blocks.pop();
	}

	//	get the number of variables in the current block of the
	//	current frame
	pub fn variable_count (&self) -> usize{
		let frame = self.operand_frames.last().unwrap();

		//	if there aren't any blocks to get the length of, return an impossible number
		if frame.operand_blocks.len() == 0{
			return 0;
		}

		let block = frame.operand_blocks.last().unwrap();
		return block.operand_block.len();
	}

	//	get the number blocks in the current frame
	pub fn block_count(&self) -> usize{
		let frame = self.operand_frames.last().unwrap();
		return frame.operand_blocks.len();
	}

	//	determines if there is a variable at the fully qualified address
	//	specified.
	//
	//	return false if there isn't one.
	pub fn exists(&self, frame_num : usize, block_num:usize, address:usize) -> bool{
		if self.operand_frames.len() == 0 {return false}
		let frame = &self.operand_frames[frame_num];
		if frame.operand_blocks.len() == 0 {return false}
		let block = &frame.operand_blocks[block_num];
		if block.operand_block.len() == 0 {return false}

		//	if the address specified is past the end of the block
		//	return false, else the var exists
		if address >= block.operand_block.len() {return false}
		true
	}

	//	same as exists but assumes the current frame
	pub fn exists_locally(&self, block_num:usize, address:usize) -> bool{
		if self.operand_frames.len() == 0 {return false}
		let frame = &self.operand_frames.last().unwrap();
		if frame.operand_blocks.len() == 0 {return false}
		let block = &frame.operand_blocks[block_num];
		if block.operand_block.len() == 0 {return false}

		//	if the address specified is past the end of the block
		//	return false, else the var exists
		if address >= block.operand_block.len() {return false}
		true
	}


	//	fetch a variable at the fully qualified address
	pub fn fetch_var(&self, frame_num : usize, block_num : usize, address : usize) -> CplVar{
		//eprintln!("{}fetch_var {},{},{}",DEBUG_INDENT, frame_num, block_num, address);
		let frame = &self.operand_frames[frame_num];
		let block = &frame.operand_blocks[block_num];
		block.operand_block[address].clone()
	}

	//	fetch a variable from a partially qualified address (i.e.
	//	in the current frame).  If the local variable is a collection
	//	return a VarRef reference to it.  We don't every want to
	//	make a copy of an array or dictionary.
	pub fn fetch_local_var(&self, block_num : usize, address : usize) -> CplVar{
		let frame = &self.operand_frames.last().unwrap();
		let frame_num = self.operand_frames.len() - 1;

		if frame.operand_blocks.len() == 0{
			abend!(format!("from CplVar.fetch_local_var: from {},{}, No operand blocks available", block_num, address));
		}

		let block_option = &frame.operand_blocks.get(block_num);

		match block_option{
			None => abend!(format!("from CplVar.fetch_local_var: from {},{}, Block Not Available. Last block is {}", block_num, address, frame.operand_blocks.len()-1)),
			Some(block) => if block.operand_block.len() == 0 {
				panic!("from CplVar.fetch_local_var: operand block {} is empty", frame.operand_blocks.len()-1);
			}else if address > block.operand_block.len()-1{
				panic!("from CplVar.fetch_local_var: address {} > operand block len {}", address, block.operand_block.len());
			}else{
				match block.operand_block[address].var{
					CplDataType::CplArray(_) |
					CplDataType::CplDict(_) => {
						return CplVar::new(CplDataType::CplVarRef(CplVarRef::new(frame_num, block_num, address)));
					},
					_ =>{}
				}
				block.operand_block[address].clone()
			}
		}
	}

	pub fn get_type(&self, data : &CplVar) -> CplDataTypeInspected{
		match data.var{
			CplDataType::CplUninitialized(_) => CplDataTypeInspected::CplUninitialized,
			CplDataType::CplUndefined(_) => CplDataTypeInspected::CplUndefined,
			CplDataType::CplNumber(_) => CplDataTypeInspected::CplNumber,
			CplDataType::CplBool(_) => CplDataTypeInspected::CplBool,
			CplDataType::CplVarRef(_) => CplDataTypeInspected::CplVarRef,
			CplDataType::CplArray(_) => CplDataTypeInspected::CplArray,
			CplDataType::CplDict(_) => CplDataTypeInspected::CplDict,
			CplDataType::CplString(_) => CplDataTypeInspected::CplString,
			_ => CplDataTypeInspected::DontCare,
		}
	}


	fn fetch_array_indexed_from_operand_stack_helper(&mut self, indices : &mut Vec<usize>) -> CplVar{
		//	a reference to the array we are wanting to index
		let mut array_ref : &CplArray;

		//	look at the top of the stack.  It needs to be a VarRef pointer
		//	to an array.  If it doesn't meet this criteria, then we're done.
		match self.operand_frames.last().unwrap().operand_blocks.last().unwrap().operand_block.last().unwrap().var{
			CplDataType::CplVarRef(ref vr) => {
				match  self.operand_frames[vr.frame_num].operand_blocks[vr.block_num].operand_block[vr.address].var{
					CplDataType::CplArray(ref a) => {
						array_ref = a;
					}
					_ =>{
						panic!("Expected to see a CplVarRef pointing at an array but didn't.  Got {}", self.operand_frames[vr.frame_num].operand_blocks[vr.block_num].operand_block[vr.address].var);
					}	
				} 
			}
			_ =>{
				panic!("Expected to see a CplVarRef but didn't. got {}. You can only index arrays.", self.operand_frames.last().unwrap().operand_blocks.last().unwrap().operand_block.last().unwrap().var);
			}	
		}

		//	Now loop through the indices until we find a scalar or we run out
		//	of indices.  If the latter and warnings have been enabled, println a warning
		//	that we are returning an array which we wouldn't normally expect to do.

		//	the indices are on the stack in the order the arrays should be accessed
		//	e.g. if the dimension is [1,2,3], the top of the stack will be 1
		while !indices.is_empty(){
			let index = indices.pop().unwrap();

			//	If the index is out of bounds, return undefined
			let element = match array_ref.cpl_array.get(index){
				None => return undefined!(),//return self.undefined(),
				Some(e) => e,
			};
			let element_type = self.get_type(element);
			match element_type{
				CplDataTypeInspected::CplArray => {}
				_ => return element.clone(),
			}

			if indices.is_empty(){
				if self.cli_warnings{
					eprintln!("Warning:  The index {} for the array {} is pointing at another array",index,array_ref);
				}
				return element.clone();
			}

			if let CplDataType::CplArray(ref a) = element.var{
				array_ref = a;
			}
		}

		undefined!()
	}


	//	Get the value from an array determined by the index (or indices).  The
	//	index count is the number of "dimensions".
	//
	//	The stack at this point is:
	//
	//		tos-index_count:  	array to index
	//		tos-n:				dimension 0
	//		  :
	//		tos-2:				dimension n-2
	//		tos-1:				dimension n-1
	//		tos:				dimension n
	//
	//	First pull the indices(dimensions) off the stack and into
	//	a local array (it's just less complicated than trying to access
	//	them directly on the stack).
	//
	//	Then call the helper to do the actual indexing.  The helper
	//	returns the final result which we assume will be a scalar
	//	because we have to clone it to return it (if it were an array that
	//	would be bad because we don't want to copy a bunch of elements in any
	//	putative loops).
	//
	//	If an index is out of bounds we return CplUndefined
	//
	pub fn fetch_array_indexed_from_operand_stack(&mut self, index_count : usize) -> CplVar{

		//	first, build an array of indices.  The last index is first and the first
		//	index is last (this is in the reverse order in which they appeared in the
		//	original CPL expression)
		let mut indices : Vec<usize> = Vec::new();
		let mut ix_num = 0;

		while ix_num < index_count{
			let ix_var = self.pop();
			if let CplDataType::CplNumber(n) = ix_var.var{
				indices.push(n.cpl_number as usize);
			}
			ix_num += 1;
		}

		let fetched = self.fetch_array_indexed_from_operand_stack_helper(&mut indices);
		
		//	The original array we are indexing is still on the stack, so we need to
		//	get rid of it
		self.pop();

		fetched.clone()
	}


	fn fetch_dict_indexed_from_operand_stack_helper(&mut self, indices : &mut Vec<String>) -> CplVar{
		//	a reference to the array we are wanting to index
		let mut dict_ref : &CplDict;

		//	look at the top of the stack.  It needs to be a VarRef pointer
		//	to an array.  If it doesn't meet this criteria, then we're done.
		match self.operand_frames.last().unwrap().operand_blocks.last().unwrap().operand_block.last().unwrap().var{
			CplDataType::CplVarRef(ref vr) => {
				match  self.operand_frames[vr.frame_num].operand_blocks[vr.block_num].operand_block[vr.address].var{
					CplDataType::CplDict(ref a) => {
						dict_ref = a;
					}
					_ =>{
						panic!("Expected to see a CplVarRef pointing at a dictionary but didn't.  Got {}", self.operand_frames[vr.frame_num].operand_blocks[vr.block_num].operand_block[vr.address].var);
					}	
				} 
			}
			_ =>{
				panic!("Expected to see a CplVarRef but didn't. got {}. You can only index arrays.", self.operand_frames.last().unwrap().operand_blocks.last().unwrap().operand_block.last().unwrap().var);
			}	
		}

		//	Now loop through the indices until we find a scalar or we run out
		//	of indices.  If the latter and warnings have been enabled, println a warning
		//	that we are returning an array which we wouldn't normally expect to do.
		while !indices.is_empty(){
			let index = &indices.pop().unwrap();
			//	If the index is out of bounds, return undefined
			let element = match dict_ref.cpl_dict.get(&CplKey::new(index)){
				None => {
					return undefined!();
				}
				Some(e) => e,
			};


			let element_type = self.get_type(element);
			match element_type{
				CplDataTypeInspected::CplDict => {}
				_ => {
					return element.clone();
				}
			}

			if indices.is_empty(){
				if self.cli_warnings{
					eprintln!("Warning:  The index {} for the dictionary {} is pointing at another dictionary",index,dict_ref);
				}
				return element.clone();
			}

			if let CplDataType::CplDict(ref a) = element.var{
				dict_ref = a;
			}
		}

		undefined!()
	}

	pub fn fetch_dict_indexed_from_operand_stack(&mut self, index_count : usize) -> CplVar{
		//	first, build an array of indices.  The last index is first and the first
		//	index is last (this is in the reverse order in which they appeared in the
		//	original CPL expression)
		let mut indices : Vec<String> = Vec::new();
		let mut ix_num = 0;

		while ix_num < index_count{
			let ix_var = self.pop();
			match ix_var.var{
				CplDataType::CplString(s) => indices.push(s.cpl_string),
				CplDataType::CplNumber(n) => indices.push(n.cpl_number.to_string()),
				CplDataType::CplBool(n) => indices.push(n.cpl_bool.to_string()),
				_=> panic!("from fetch_dict_indexed_from_operand_stack: {} is not a valid key",ix_var),
			}
			ix_num += 1;
		}

		let fetched = self.fetch_dict_indexed_from_operand_stack_helper(&mut indices);
		
		//	The original array we are indexing is still on the stack, so we need to
		//	get rid of it
		self.pop();

		fetched.clone()
	}

	//	Return an element of an array
	pub fn fetch_indexed (&mut self, index : usize, block_num : usize, address : usize) -> CplVar{
		let var = self.fetch_local_var(block_num, address);
		if let CplDataType::CplVarRef(ref varref) = var.var{
			let array_var = self.operand_frames.get_mut(varref.frame_num).unwrap().operand_blocks.get(varref.block_num).unwrap().operand_block.get(varref.address).unwrap();

			if let CplDataType::CplArray(ref array) = array_var.var{
				return array.cpl_array[index].clone();
			}else{
				panic!("from OperandStack.fetch_indexed: expecting a VarRef pointing at array, Got:{}", array_var);
			}
		}else{
			panic!("from OperandStack.fetch_indexed: expecting a VarRef, Got:{}", var);
		}

	}
	

	//	Returns a reference to the top of the stack
	pub fn fetch_tos_ref(&self) -> &CplVar{
		self.operand_frames.last().unwrap().operand_blocks.last().unwrap().operand_block.last().unwrap()
	}

	pub fn fetch_mutable_tos_ref(&mut self) -> &mut CplVar{
		self.operand_frames.last_mut().unwrap().operand_blocks.last_mut().unwrap().operand_block.last_mut().unwrap()
	}

	pub fn fetch_ref(&self, frame_num : usize, block_num : usize, address : usize) -> &CplVar{
		self.operand_frames.get(frame_num).unwrap().operand_blocks.get(block_num).unwrap().operand_block.get(address).unwrap()
	}

	pub fn fetch_mutable_ref (&mut self, frame_num : usize, block_num : usize, address : usize) -> &mut CplVar{
		self.operand_frames.get_mut(frame_num).unwrap().operand_blocks.get_mut(block_num).unwrap().operand_block.get_mut(address).unwrap()
	}

	pub fn fetch_local_ref(&self, block_num : usize, address : usize) -> &CplVar{
		let frame_num = self.current_frame();
		self.operand_frames.get(frame_num).unwrap().operand_blocks.get(block_num).unwrap().operand_block.get(address).unwrap()
	}

	pub fn fetch_local_mutable_ref (&mut self, block_num : usize, address : usize) -> &mut CplVar{
		let frame_num = self.current_frame();
		self.operand_frames.get_mut(frame_num).unwrap().operand_blocks.get_mut(block_num).unwrap().operand_block.get_mut(address).unwrap()
	}

	pub fn update_local(&mut self, var : &CplVar, target_block_num:usize, target_address:usize){
		let target_frame_num = self.operand_frames.len()-1;
		self.update_from_val(var, target_frame_num, target_block_num, target_address);
	}

	pub fn update_local_from_tos(&mut self, target_block_num:usize, target_address:usize){
		let target_frame_num = self.operand_frames.len()-1;
		self.update_from_tos(target_frame_num, target_block_num, target_address);
	}

	//	update the value at the fully qualified address specified with the var at the top
	//	of the stack and then remove it.  Assumes the current frame.
	pub fn update_from_tos (&mut self, target_frame_num:usize, target_block_num:usize, target_address:usize){
		//self.dump_operands_with_message("=========== update_from_tos ========");
		let tos = self.pop();

		self.update_from_val(&tos, target_frame_num, target_block_num, target_address);
	}


	//	update the value at the fully qualified address specified with the var at the top
	//	of the stack and then remove it.  Assumes the current frame.
	fn update_from_val (&mut self, rvalue : &CplVar, target_frame_num:usize, target_block_num:usize, target_address:usize){
		//	get the frame of the target
		let target_frame = self.operand_frames.get_mut(target_frame_num).unwrap();

		//	get the block specified
		let target_block = target_frame.operand_blocks.get_mut(target_block_num).unwrap();

		let target_interner = target_block.operand_block[target_address].interner;
		
		//	and update the value at target_frame.target_block.target_address
		target_block.operand_block[target_address] = CplVar::inew(rvalue.var.clone(), target_interner);
	}

	
	pub fn current_block_num(&self) -> usize{
		let operand_frame : &OperandFrame = self.operand_frames.last().unwrap();
		return operand_frame.operand_blocks.len()-1;
	}

	//	creates (or reuses) an uninitialized variable in the current block
	//	DESIGN NOTE:  I can't think of a reason to allocate space anywhere
	//	else in the operand stack.
	pub fn alloc(&mut self, block_num:usize, address:usize){
		self.named_alloc(block_num, address, 0);
	}

	pub fn named_alloc(&mut self, block_num:usize, address:usize, interner : usize){
		//	if the location already exists then we're done
		if self.exists_locally(block_num, address){
			return;
		}

		//	so we need to create a new variable in the current block and frame
		let frame = self.operand_frames.last_mut().unwrap();

		//	get the current block
		let block = frame.operand_blocks.get_mut(block_num).unwrap();

		//	create a new uninitialized variable at the top of the block
		block.operand_block.push(CplVar::inew(CplDataType::CplUninitialized(CplUninitialized::new()),interner));


		//	Another sanity check.  We expect address of this new var to be the
		//  address requested
		if address != block.operand_block.len() - 1{
			panic!("from CplVar.alloc:  address of the new variable is not what was requested {} != {}", address, block.operand_block.len() - 1);
		}
	}


	//	pushes the value at tos onto the array at tos-1.  Removes the
	//	value being pushed
	pub fn push_array_element(&mut self, interner : usize){
		//	get the element to add to the array
		let mut element = self.dereference_tos();
		element.interner = interner;

		//	get a reference to the array at the top of stack
		let array = self.fetch_mutable_tos_ref();


		//	now get the underlying array
		if let CplDataType::CplArray(ref mut a) = array.var{
			a.cpl_array.push(element);
			//a.push(&element);
		}else{
			panic!("from OperandStack.push_array_element:  expecting an Array at the top of the stack.  Got {}",array);
		}
	}

	//	inserts the key (tos-1)/value(tos) into a dictionary at tos-2
	pub fn insert_dict (&mut self){		
		//	get the element to add to the array
		let value = self.pop();
		let key = self.pop();
		let mut dict = self.pop();

		//	now get the underlying dictionary and insert the key/value pair
		if let CplDataType::CplDict(ref mut a) = dict.var{
			a.cpl_dict.insert(CplKey::to_key(&key.var),value);
		}

		//	add the dictinary back to the operands
		self.push(&dict);		
	}

	//	Perform assignment opperator on a scalar value (e.g. x += 1);
	pub fn apply_binary_operator_scalar (&mut self, block_num : usize, address : usize, opcode : Opcode){
		//	get the new value from the top of the stack	
		let tos = self.dereference_tos();

		//	compute the address of the target (which we know is a scalar)
		let frame = self.operand_frames.last_mut().unwrap();
		let block = frame.operand_blocks.get_mut(block_num).unwrap();
		let var = block.operand_block.get_mut(address).unwrap(); 

		//	What we don't know yet, is if it is a number or string.  If it's a
		//	number, we an apply the operator directly (which will complain if the operator
		//	isn't actually a numeric operator).  If it's a string, we hand things off to
		//	the CplString data type to figure out what to do with it.
		//
		//	NOTE: even if it's a string that looks like a number, and the expression is
		//	a number, the result is still a string but with its "numberic" value changed
		match var.var{
			CplDataType::CplNumber (ref mut n) => n.apply_binary_operator_to_number(&tos, opcode),
			CplDataType::CplString (ref mut s) => s.apply_binary_operator_to_string(&tos, opcode),
			_=> abend!(format!("from CplVar.apply_binary_operator_scalar:  unable to perform {} on type {}", opcode, var.var)),
		}		
	}

	//	Perform assignment opperator on an element of a collection (e.g. x[0] += 1);
	//	NOTE this is for direct access to the collection (i.e. not via a VarRef)
	pub fn apply_binary_operator_indexed (&mut self, block_num : usize, address : usize, opcode : Opcode){
		//	get the new value from the top of the stack
		let rvalue = self.dereference_tos();

		//  get the index from the stack
		let index = self.dereference_tos();

		//	get a reference to the target.  It is either a collection or a VarRef (in which
		//	case we are updating an element of a collection)
		match self.operand_frames.last_mut().unwrap().operand_blocks.get_mut(block_num).unwrap().operand_block.get_mut(address).unwrap().var{
			CplDataType::CplArray (ref mut a) =>{
				a.update_indexed_op(&index, &rvalue, opcode);
			}

			CplDataType::CplDict (ref mut d) => {
				d.update_indexed_op(&index, &rvalue, opcode);
			}
						
			_=> abend!(format!("from CplVar.apply_binary_operator_indexed:  unable to perform {}", opcode)),
		}		
	}

	//	updates the value of a local array element whose index is at tos-1 with
	//	the new value at tos	
	pub fn update_local_collection(&mut self, block_num : usize, address: usize){
		let value = self.dereference_tos();
		let index_var = self.dereference_tos();

		//	get the current frame
		let frame = self.operand_frames.last_mut().unwrap();

		//	get array at the address
		let collection = frame.operand_blocks.get_mut(block_num).unwrap().operand_block.get_mut(address).unwrap();
		if let CplDataType::CplArray(ref mut a) = collection.var{
			a.update_indexed(&index_var, &value);
		} else if let CplDataType::CplDict(ref mut d) = collection.var{
			d.update_indexed(&index_var, &value);
		}else{
			abend!(format!("from OperandStack.update_local_collection:  collection needs to be an array or dictionary.  It was {}", collection.var));
		}
	}

	//	given a CplVar return its type as a string
	pub fn get_cpl_type(&mut self, var : &CplVar) -> String{
		let rtn : &str;

		match var.var{
			CplDataType::CplNumber(_) 				=> rtn = "CplNumber",
			CplDataType::CplString(_)				=> rtn = "CplString",
			CplDataType::CplBool(_)					=> rtn = "CplBool",
			CplDataType::CplUninitialized(_) 		=> rtn = "CplUnitialized",
			CplDataType::CplUndefined(_) 			=> rtn = "CplUndefined",
			CplDataType::CplArray(_) 				=> rtn = "CplArray",
			CplDataType::CplVarRef(_)				=> rtn = "CplVarRef",
			CplDataType::CplFileReader(_) 			=> rtn = "CplFileHandle",
			CplDataType::CplFileWriter(_) 			=> rtn = "CplFileHandle",
			CplDataType::CplFileAppender(_) 		=> rtn = "CplFileHandle",
			CplDataType::CplDict(_) 				=> rtn = "CplDict",
			CplDataType::CplStruct(_)				=> rtn = "CplStruct",
		}

		return rtn.to_string();
	}

	//	Does a unary operatorion on the local variable specified by the address
	//	But if the variable is a VarRef, the actual location can be anywhere
	//	in the operand stack.
	pub fn perform_unary_op(&mut self, _address: usize, opcode : Opcode){
		
		//	get the frame and block
		// let frame = self.operand_frames.last_mut().unwrap();
		// let block = frame.operand_blocks.last_mut().unwrap();
		// let var = block.operand_block.get_mut(address).unwrap(); 

		let mut var = self.pop();
		match var.var{
			CplDataType::CplNumber (ref mut n) => n.apply_unary_operator(opcode),
			CplDataType::CplBool(ref mut b) => b.apply_daminit(),
			_=> abend!(format!("from CplVar.perform_unary_op:  unable to perform {} on type {}", opcode, var.var)),
		}

		self.push(&var);
	}

	// pub fn operand_list_text(&self) -> String{
	// 	if self.operand_frames.is_empty(){
	// 		return "Operands: NONE - No Frames".to_string();
	// 	}
	// 	let mut rtn = "Operands\n".to_string();

	// 	let mut i = 0;
	// 	while i < self.operand_frames.len(){
	// 		rtn.push_str(&self.operand_frames[i].operand_list_text());
	// 		i+=1;
	// 	}
	// 	rtn.clone()
	// }


	//	Requested the length of a variable pointed to by a VarRef
	//	Follow it down the rabbit hole until we see a real variable
	//	and then return its length.
	fn varref_len(&mut self, varref : &CplVar) -> usize{
		let mut local_varref = varref;
		loop{
			match local_varref.var{
				CplDataType::CplArray(ref a)	=> return a.len(),
				CplDataType::CplString(ref s)	=>return  s.len(),
				CplDataType::CplDict(ref d)		=> return d.len(),
				CplDataType::CplVarRef(ref vr)		=>{
					local_varref = &self.operand_frames.get(vr.frame_num).unwrap()
						.operand_blocks.get(vr.block_num).unwrap()
						.operand_block.get(vr.address).unwrap();
					continue;
				}
				_=> return 1,
			}
		}
	}

	//	get the length of a variable:
	//
	//		Array:  number of elements
	//		String:  number of characters
	//		Dictionary: number of keys
	//		All other types: 1
	pub fn len(&mut self, var : &CplVar) -> usize{
		match var.var{
			CplDataType::CplVarRef(_) 		=> {
				return self.varref_len(var);
			}
			CplDataType::CplArray(ref a)	=> return a.len(),
			CplDataType::CplString(ref s)	=> return s.len(),
			CplDataType::CplDict(ref d)		=> return d.len(),
			_=> return 1,
		};
	}

	pub fn dump_operands_with_message(&self, msg : &str){
		eprintln!("OPERAND STACK DUMP : {}", msg);
		self.dump_operands_without_title();
	}

	pub fn dump_operands(&self){
		eprintln!("--- OPERAND STACK DUMP --\n");
		self.dump_operands_without_title();
	}

	//	We have to do this here because CplArray doesn't know what the namelist
	//	is so "display for array" won't work
	fn dump_array(&self, array : &CplArray){
		let mut begin = true;
		for element in &array.cpl_array{
			if element.interner > 0 {
				if begin{
					eprint!("{} ({})",element, self.namelist[element.interner]);
					begin = false;
				}else{
					eprint!(",{} ({})",element, self.namelist[element.interner]);
				}
			}else{
				if begin{
					eprint!("{} ()",element);
					begin = false;
				}else{
					eprint!(",{} ()",element);
				}
			}
		}
		eprint!("\n");
	}

	pub fn dump_operands_without_title(&self){
		if self.operand_frames.len() == 0{
			eprintln!("No Frames");
			return;
		}
		
		let mut frame_num = 0;
		while frame_num < self.operand_frames.len(){
			let block_count = self.operand_frames[frame_num].operand_blocks.len();
			if block_count == 0{
				eprintln!("   No blocks in frame# {}",frame_num);
			}else{
				eprintln!("Frame# {} Block Count={}", frame_num, block_count);
				let mut block_num = 0;
				while block_num < block_count{
					// eprintln!("   Block# {}", block_num);
					let variable_count = self.operand_frames[frame_num].operand_blocks[block_num].operand_block.len();
					if variable_count == 0{
						eprintln!("   No variables in block# {}", block_num);
					}else{
						eprintln!("   Block# {} Variable count={}", block_num, variable_count);
						let mut variable_num = 0;
						while variable_num < variable_count{
							let variable = &self.operand_frames[frame_num].operand_blocks[block_num].operand_block[variable_num];

							if let CplDataType::CplArray(ref array) = variable.var{
								eprint!("      ");
								self.dump_array(array);
							}else{
								eprintln!("      {}: {} ({})",variable_num, variable, self.namelist[variable.interner]);
							}
							variable_num += 1;
						}	
					}
					block_num += 1;
				}
			}
			frame_num += 1;
		}
		eprintln!("");
	}

	fn get_data_loc_from_ref(&self, var_ref : &CplVar) -> (usize, usize, usize){
		if let CplDataType::CplVarRef(ref vr) = var_ref.var{
			return (vr.frame_num, vr.block_num, vr.address);
		}else{
			abend!(format!("From get_frame_num_from_ref: Expecting a VarRef, got {}", var_ref.var));
		}
	}

	pub fn dereference(&mut self, var : &CplVar) -> CplVar{
		let mut rtn = var.clone();
		loop{
			if let CplDataType::CplVarRef(_) = rtn.var{
				let data_loc = self.get_data_loc_from_ref(&rtn);
				let frame_num = data_loc.0;
				let block_num = data_loc.1;
				let address = data_loc.2;
				let operand_frame = self.operand_frames.get_mut(frame_num).unwrap();
				let operand_block = operand_frame.operand_blocks.get_mut(block_num).unwrap();
				rtn = operand_block.operand_block.get_mut(address).unwrap().clone();
				if let CplDataType::CplVarRef(_) = rtn.var{
					continue;
				}
			}else{
				break;
			}
		}
		return rtn.clone();
	}


	//	return a reference to the var pointed to by a VarRef
	// pub fn dereference_to_ref(&self) -> &CplVar{

	// }
	//	pop the VarRef at the top of the stack and if it's a CplVarRef return
	//	the actual value otherwise just return what got popped without any fuss.  NOTE:
	//	if the actual value is also a VarRef we need to recurse until we get to
	//	the real thing.  But so far, I haven't seen an alloc for a VarRef
	pub fn dereference_tos(&mut self) -> CplVar{
		let var = self.pop();
		self.dereference(&var)
	}
}

/****************************************
****	Operand Frame
*****************************************/
pub struct OperandFrame{
	//cli : & 'a CLI<'a>,
	pub operand_blocks : Vec<OperandBlock>,
}

impl OperandFrame{
	pub fn new() -> OperandFrame{
		OperandFrame{
			//cli : cli,
			operand_blocks : Vec::new(),
		}
	}
	pub fn operand_list_text(&self) -> String{
		if self.operand_blocks.is_empty(){
			return "No Blocks".to_string();
		}
		let mut rtn = "Operands\n".to_string();

		let mut i = 0;

		while i < self.operand_blocks.len(){
			if self.operand_blocks.is_empty(){
				rtn.push_str(&format!("   Block {} NONE\n", i));
			}else{
				rtn.push_str(&format!("   Block: {}\n", i));
				rtn.push_str(&self.operand_blocks[i].operand_list_text());
			}		
			i += 1;
		}
		rtn.clone()
	}
}

/****************************************
****	Operand Block
*****************************************/

//	An operand block is a partition of the operand stack created for
//	IF, WHILE, ELSE, etc.  When a function begins executing one of these
//	is created.  Variables in a frame are accessed via frame number and index.
pub struct OperandBlock{
	//cli : & 'a CLI<'a>,
	pub operand_block : Vec<CplVar>,
}

impl OperandBlock{
	pub fn new() -> OperandBlock{
		OperandBlock{
			//cli : cli,
			operand_block: Vec::new(),
		}
	}

	pub fn operand_list_text(&self) -> String{
		let mut rtn = String::new();
		let mut i=0;
		for var in &self.operand_block{
			match &var.var{
				CplDataType::CplArray(array) => rtn.push_str(&array.array_text()),
				_=> rtn.push_str(&format!("       {}: {}\n", i, var.var)),
			}
			i += 1;
		}
		rtn.clone()
	}
}


/****************************************
****	CplVar
*****************************************/
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct CplVar{
	pub var : CplDataType,
	pub interner : usize,
}

impl CplVar{
	//	This constructor sets the interner to 0
	pub fn new(var : CplDataType) ->CplVar{
		CplVar{
			var : var,
			interner : 0,
		}
	}

	//	This constructor includes the interner value and so can
	//	set it
	pub fn inew(var : CplDataType, interner : usize) -> CplVar{
		CplVar{
			var : var,
			interner : interner,
		}
	}

	pub fn is_type_equal (&self, v : &CplVar) -> bool{
		match v.var{
			CplDataType::CplUninitialized(_) => if let CplDataType::CplUninitialized(_) = self.var {return true}else{false},
			CplDataType::CplUndefined(_) => if let CplDataType::CplUndefined(_) = self.var {return true}else{return false},
			CplDataType::CplNumber(_) => if let CplDataType::CplNumber(_) = self.var {return true}else{return false},
			CplDataType::CplBool(_) => if let CplDataType::CplBool(_) = self.var {return true}else{return false},
			CplDataType::CplVarRef(_) => if let CplDataType::CplVarRef(_) = self.var {return true}else{return false},
			CplDataType::CplArray(_) => if let CplDataType::CplArray(_) = self.var {return true}else{return false},
			CplDataType::CplDict(_) => if let CplDataType::CplDict(_) = self.var {return true}else{return false},
			CplDataType::CplString(_) => if let CplDataType::CplString(_) = self.var {return true}else{return false},
			CplDataType::CplFileReader(_) => if let CplDataType::CplFileReader(_) = self.var {return true}else{return false},
			CplDataType::CplFileWriter(_) => if let CplDataType::CplFileWriter(_) = self.var {return true}else{return false},
			CplDataType::CplFileAppender(_) => if let CplDataType::CplFileAppender(_) = self.var {return true}else{return false},
			CplDataType::CplStruct(_) => if let CplDataType::CplStruct(_) = self.var {return true}else{return false},
		}
	}

	fn is_disallowed(&self, v : &CplVar, warn : bool) -> bool{
		match self.var{
			CplDataType::CplUninitialized(_) |
			CplDataType::CplUndefined(_) |
			CplDataType::CplVarRef(_) |
			CplDataType::CplArray(_) |
			CplDataType::CplDict(_) |
			CplDataType::CplFileReader(_) |
			CplDataType::CplFileWriter(_) |
			CplDataType::CplFileAppender(_) => {
				if warn{
					eprintln!("Warning: unable compare {} with {}", self.var, v.var);
				}
				return true;
			},
			_ => return false,
		}
	}

	
	//	Compare the value of this instance with another instance.  Only Numbers, Booleans and Strings
	//	can be compared (we'll add arrays later).  If the types are not the same then
	//	an attempt is made to convert the argument to the same type as this instance.  For example
	//	if this instance is a string and a number is passed, the number is turned into a string.  If this
	//	instance is a number and a string is passed, at attempt is made to turn it into a number
	//	and then compared.  If the conversion fails then false is returned.
	//
	//	If the warn flag is true, messages about type mismatch and failed conversions are printed
	//	these do not abend the program.

	pub fn is_equal (&self, v : &CplVar, warn : bool) -> bool{

		//	filter out any types that can't be compared
		if self.is_disallowed(&self, warn) {return false}
		if self.is_disallowed(v, warn) {return false}

		//	If the types are equal, just go ahead and compare their values
		if self.is_type_equal(v){
			match &v.var{
				CplDataType::CplNumber(n1) => if let CplDataType::CplNumber(ref n2) = self.var {return n1.cpl_number == n2.cpl_number}else{return false},
				CplDataType::CplBool(b1) => if let CplDataType::CplBool(ref b2) = self.var {return b1.cpl_bool == b2.cpl_bool}else{return false},
				CplDataType::CplString(s1) => if let CplDataType::CplString(ref s2) = self.var {return s1.cpl_string == s2.cpl_string}else{return false},
				_ => if warn {
					eprintln!("Warning from CplVar.is_equal:  I don't know what this is {}", v.var);
				},
			}
		}
		return false;
	}

	pub fn is_not_equal (&self, v : &CplVar, warn : bool) -> bool{

		//	filter out any types that can't be compared
		if self.is_disallowed(&self, warn) {return false}
		if self.is_disallowed(v, warn) {return false}

		//	If the types are equal, just go ahead and compare their values
		if self.is_type_equal(v){
			match &v.var{
				CplDataType::CplNumber(n1) => if let CplDataType::CplNumber(ref n2) = self.var {return n1.cpl_number != n2.cpl_number}else{return false},
				CplDataType::CplBool(b1) => if let CplDataType::CplBool(ref b2) = self.var {return b1.cpl_bool != b2.cpl_bool}else{return false},
				CplDataType::CplString(s1) => if let CplDataType::CplString(ref s2) = self.var {return s1.cpl_string != s2.cpl_string}else{return false},
				_ => if warn {
					eprintln!("Warning from CplVar.is_equal:  I don't know what this is {}", v.var);
				},
			}
		}
		return false;
	}


	pub fn as_string(&self) -> String{
		match &self.var{
			CplDataType::CplString(s) => return s.cpl_string.clone(),
			CplDataType::CplNumber(n) => return n.cpl_number.to_string(),
			CplDataType::CplBool(b) => return b.cpl_bool.to_string(),
			_ => panic!("From CplVar.as_string: Var isn't a string, number or boolean it's a {}", self.var),
		}
	}

	pub fn as_number(&self) -> f64{
		match &self.var{
			CplDataType::CplNumber(s) => return s.cpl_number,
			CplDataType::CplString(s) => {
				match s.cpl_string.parse::<f64>(){
					Ok(n) => return n,
					Err(_) => abend!(format!("From CplString.as_number: couldn't covert {} into a number", s.cpl_string)),
				}
			},
			_ => abend!(format!("From CplVar.as_scalar: Var is not a scalar, it's a {}", self.var)),
		}
	}

	pub fn as_boolean(&self) -> bool{
		match &self.var{
			CplDataType::CplBool(s) => return s.cpl_bool,
			_ => abend!(format!("From CplVar.as_string: Var is not a boolean, it's a {}", self.var)),
		}
	}

	pub fn len(&self, _of_what : &CplVar) -> usize{
		return 0;
	}

	pub fn get_keys(&self) -> Vec<String>{
		return vec!("akey".to_string());
	}

	pub fn print(&self){
		match &self.var{
			CplDataType::CplNumber(n) 				=> eprintln!("{}",n.cpl_number),
			CplDataType::CplString(s)				=> eprintln!("{}",s.cpl_string),
			CplDataType::CplBool(b)					=> eprintln!("{}",b.cpl_bool),
			CplDataType::CplVarRef(v)				=> eprintln!("VarRef: {},{},{}", v.frame_num, v.block_num, v.address),
			CplDataType::CplArray(a)				=> a.print(),
			CplDataType::CplUninitialized(_) 		=> eprintln!("Uninitialized"),
			CplDataType::CplUndefined(_) 			=> eprintln!("Undefined"),
			CplDataType::CplDict(_) 				=> eprintln!("Dictionary"),
			CplDataType::CplFileReader(_) 			=> eprintln!("File Reader"),
			CplDataType::CplFileWriter(_) 			=> eprintln!("File Writer"),
			CplDataType::CplFileAppender(_) 		=> eprintln!("File Appender"),
			CplDataType::CplStruct(a)				=> a.print(),
		}
	}

	//	return a hacked up version of this var used by trace displays and other
	//	debugging purposes.  If it's not a string just return it.  If it is a string, make sure
	// 	replace control codes with "//<code>"
	//	codes in it
	pub fn dbg(&self) -> String{
		if let CplDataType::CplString(ref s) = self.var{
			let re = Regex::new(r"([\r])").unwrap();
			let result = re.replace_all(&s.cpl_string, r"\r");
			return format!("{}",result);
		}else{
			return format!("{}",self.var);
		}
	}
}

impl fmt::Display for CplVar{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match &self.var{
			CplDataType::CplNumber(n) 			=> write!(f,"{}",n.cpl_number),
			CplDataType::CplString(s)			=> write!(f,"{}",s.cpl_string),
			CplDataType::CplBool(b)				=> write!(f,"{}",b.cpl_bool),
			CplDataType::CplVarRef(v)			=> write!(f,"VarRef: {},{},{}", v.frame_num, v.block_num, v.address),
			CplDataType::CplArray(a)			=> write!(f,"[{}]",a),
			CplDataType::CplUninitialized(_) 	=> write!(f,"Uninitialized"),
			CplDataType::CplUndefined(_) 		=> write!(f,"Undefined"),
			CplDataType::CplDict(_) 			=> write!(f,"Dictionary"),
			CplDataType::CplFileReader(_) 		=> write!(f,"File Reader"),
			CplDataType::CplFileWriter(_) 		=> write!(f,"File Writer"),
			CplDataType::CplFileAppender(_) 	=> write!(f,"File Appender"),
			CplDataType::CplStruct(_)			=> write!(f,"Struct"),
		}	
	}
}

impl Clone for CplVar{
	fn clone(&self) -> CplVar{
		CplVar{
			var : self.var.clone(),
			interner : self.interner,
		}
	}
}


/****************************************
****	CplData
*****************************************/
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CplVarRef{
	pub frame_num : usize,
	pub block_num : usize,
	pub address : usize,
}
impl CplVarRef{
	pub fn new(frame_num : usize, block_num : usize, address : usize) -> CplVarRef{
		CplVarRef{
			frame_num : frame_num,
			block_num : block_num,
			address : address,
		}
	}

	// pub fn update_via_ref(&mut self, cpl_var : &CplVar, operand_stack : & mut OperandStack){
	// 	let operand_frame = operand_stack.operand_frames.get_mut(self.frame_num).unwrap();
	// 	let operand_block = operand_frame.operand_blocks.get_mut(self.block_num).unwrap();
	// 	operand_block.operand_block[self.address] = cpl_var.clone();
	// }

	pub fn apply_binary_operator_indexed(&mut self, _operand_stack : &OperandStack, _rvalue : &CplVar, _index : &CplVar, _opcode : Opcode){

		eprintln!("================ CplVarRef:apply_binary_operator_indexed!!")

	}
}

impl fmt::Display for CplVarRef{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "VarRef: {},{},{}", self.frame_num, self.block_num, self.address)
	}
}


#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct CplUninitialized{
//	dummy : String,
}
impl CplUninitialized{
	pub fn new() -> CplUninitialized{
		CplUninitialized{
			// dummy : "Uninitialized Data".to_string(),
		}
	}
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct CplUndefined{
	//dummy : String,
}
impl CplUndefined{
	pub fn new() -> CplUndefined{
		CplUndefined{
			//dummy : "Undefined Data".to_string(),
		}
	}
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct CplArray{
	pub cpl_array : Vec<CplVar>,
}

impl CplArray{
	pub fn new() -> CplArray{
		CplArray{
			cpl_array : Vec::new(),
		}
	}

	pub fn push(&mut self, item : &CplVar){
		self.cpl_array.push(item.clone());
	}

	pub fn pop(&mut self) -> CplVar{
		self.cpl_array.pop().unwrap()
	}

	pub fn get (&mut self, index : usize) -> CplVar{
		if index >= self.cpl_array.len() {
			return undefined!();
		}
		self.cpl_array.get(index).unwrap().clone()
	}

	pub fn fetch_indexed(&mut self, index : &CplVar) -> CplVar{
		if let CplDataType::CplNumber(ref n) = index.var{
			if n.cpl_number < 0.0 {
				abend!(format!("From CplArray.fetch_indexed Index is negative {}", n.cpl_number));
			}
			return self.get(n.cpl_number as usize);
		}else{
			abend!(format!("from CplArray.fetch_indexed: index should be a number.  It is a {}", index.var));
		}
	}

	//	update the element at index
	pub fn update_indexed(&mut self, index : &CplVar, rvalue : &CplVar){
		if let CplDataType::CplNumber(ref n) = index.var{
			if n.cpl_number as usize >= self.cpl_array.len(){
				abend!(format!("From CplArray.update_indexed:  index is out of range"));
			}else{
				self.cpl_array[n.cpl_number as usize] = rvalue.clone();
			}
		}
	}

	//	We'll treat append as kind of special case since it only works with strings
	fn update_indexed_append(&mut self, index : usize, rvalue : &CplVar){
		if let CplDataType::CplString(ref s) = self.cpl_array[index].var{
			let mut updated_value = s.cpl_string.clone();
			if let CplDataType::CplString(ref new_s) = rvalue.var{
				updated_value.push_str(&new_s.cpl_string);
			}else if let CplDataType::CplNumber(ref new_n) = rvalue.var{
				updated_value.push_str(&new_n.cpl_number.to_string());
			}
			self.cpl_array[index] = CplVar::new(CplDataType::CplString(CplString::new(updated_value)));
		}else{
			panic!("from CplArray.update_indexed_append:  Can only append a string to a string.  Element is: {}",self.cpl_array[index].var);
		}
	}

	fn update_indexed_op_number(&mut self, index : usize, raw_value : f64){
		self.cpl_array[index] = CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Real, raw_value)));
	}

	//	Perform an operation on an element
	pub fn update_indexed_op(&mut self, index : &CplVar, rvalue : &CplVar, op : Opcode){
		let local_index : usize;

		//	otherwise, first make sure the index is in range and is a number
		if let CplDataType::CplNumber(ref n) = index.var{
			if n.cpl_number as usize >= self.cpl_array.len(){
				abend!(format!("From CplArray.update_indexed_op:  index {} is out of range.  ", n.cpl_number));
			}else{
				local_index = n.cpl_number as usize;
			}
		}else{
			abend!(format!("From CplArray.update_indexed_op:  index expected to be a number.  It is a {}", index.var));
		}

		if let CplDataType::CplNumber(ref new_v) = rvalue.var{
			if let CplDataType::CplNumber(ref e) = self.cpl_array[local_index].var{
				match op{
					Opcode::AddEq => self.update_indexed_op_number(local_index, e.cpl_number+new_v.cpl_number),
					Opcode::SubEq => self.update_indexed_op_number(local_index, e.cpl_number-new_v.cpl_number),
					Opcode::DivEq => self.update_indexed_op_number(local_index, e.cpl_number/new_v.cpl_number),
					Opcode::MulEq => self.update_indexed_op_number(local_index, e.cpl_number*new_v.cpl_number),
					Opcode::ModEq => self.update_indexed_op_number(local_index, (e.cpl_number as i64%new_v.cpl_number as i64) as f64),
					Opcode::OrEq  => self.update_indexed_op_number(local_index, (e.cpl_number as i64|new_v.cpl_number as i64) as f64),
					Opcode::AndEq => self.update_indexed_op_number(local_index, (e.cpl_number as i64&new_v.cpl_number as i64) as f64),
					_=> abend!(format!("from CplArray:update_indexed_op: Expecting an arithmetic assignment operator (e.g. '+=').  Got {}", op)),
				}
			}else if let CplDataType::CplString(_) = self.cpl_array[local_index].var{
				//	Can only apply "." or "+" to a string
				if op != Opcode::AppendEq && op != Opcode::AddEq {
					panic!("from CplArray:update_indexed_op: Op {} only works on numbers. Array element is {}", op, self.cpl_array[local_index].var);
				}
				self.update_indexed_append(local_index, rvalue);
			}else{
				abend!(format!("from CplArray:update_indexed_op: Op {} only works on numbers. Array element is {}", op, self.cpl_array[local_index].var));
			}
		}else if let CplDataType::CplString(ref new_v) = rvalue.var{
			if let CplDataType::CplString(ref mut s) = self.cpl_array[local_index].var{
				s.cpl_string.push_str(&new_v.cpl_string);
			}else{
				abend!(format!("from CplArray:update_indexed_op: Op {} only works on strings. Array element is {}", op, self.cpl_array[local_index].var));
			}
		}
	}

	pub fn delete(&mut self, key : &CplVar){
		if let CplDataType::CplNumber(ref n) = key.var{
			self.cpl_array.remove(n.cpl_number as usize);
		}
		abend!(format!("Deleting values from an array requires a number.  Found: {}", key.var));
	}

	pub fn append(&mut self, appendee : &CplVar){
		match &appendee.var{
			CplDataType::CplArray(a) =>{
				for element in &a.cpl_array{
					self.cpl_array.push(element.clone());
				}
			},
			CplDataType::CplNumber(_) | CplDataType::CplString(_) | CplDataType::CplBool(_) => return self.push(appendee),
			_ => abend!(format!("Unable to append {} to an array", appendee.var)),
		}
	}

	pub fn apply_binary_operator_to_array (&mut self, rvalue : &CplVar, op : Opcode, ){
		//	Only concat or add are valid at this point in the history of the universe.
		//	Maybe we will consider "matrix" operations.
		if op != Opcode::AppendEq && op != Opcode::AddEq{
			panic! ("from CplArray.apply_binary_operator_to_array:  Only '+' or '.' is allowed (for now).  Got {}",op);
		}

		//	if the new value is an array then it's just append (see above)
		if let CplDataType::CplArray(_) = rvalue.var{
			self.append(rvalue);
		}else{
			self.cpl_array.push(rvalue.clone());
		}
	}

	pub fn get_type_at_index(&self, index : usize) -> CplDataTypeInspected{
		match self.cpl_array[index].var{
			CplDataType::CplUninitialized(_) => CplDataTypeInspected::CplUninitialized,
			CplDataType::CplUndefined(_) => CplDataTypeInspected::CplUndefined,
			CplDataType::CplNumber(_) => CplDataTypeInspected::CplNumber,
			CplDataType::CplBool(_) => CplDataTypeInspected::CplBool,
			CplDataType::CplVarRef(_) => CplDataTypeInspected::CplVarRef,
			CplDataType::CplArray(_) => CplDataTypeInspected::CplArray,
			CplDataType::CplDict(_) => CplDataTypeInspected::CplDict,
			CplDataType::CplString(_) => CplDataTypeInspected::CplString,
			_ => CplDataTypeInspected::DontCare,
		}
	}


	pub fn sort(&mut self){
		self.cpl_array.sort();
	}

	pub fn len(&self) -> usize{
		self.cpl_array.len()
	}

	pub fn clear(&mut self){
		self.cpl_array.clear();
	}

	pub fn print(&self){
		eprintln!("Array len: {}", self.cpl_array.len());

		let mut i = 0;
		for var in &self.cpl_array{
			eprintln!("    {}:{}", i, var.var);
			i += 1;
		}
	}

	pub fn array_text(&self) -> String{
		let mut rtn = String::new();
		for var in &self.cpl_array{
			rtn.push_str(&format!("[{}], ",var.var));
		}
		if rtn.len() > 1{
			rtn.truncate(rtn.len()-2);
		}
		rtn.clone()
	}
}

impl Clone for CplArray{
	fn clone(&self) -> CplArray{
		let mut cpl_array  = CplArray::new();
		
		for var in &self.cpl_array{
			match &var.var{
				CplDataType::CplNumber(n) 				=> cpl_array.push(&CplVar::inew(CplDataType::CplNumber(CplNumber::new(n.rust_data_type, n.cpl_number)),var.interner)),
				CplDataType::CplString(n)				=> cpl_array.push(&CplVar::inew(CplDataType::CplString(CplString::new(n.cpl_string.clone())),var.interner)),
				CplDataType::CplBool(n)					=> cpl_array.push(&CplVar::inew(CplDataType::CplBool(CplBool::new(n.cpl_bool)),var.interner)),
				CplDataType::CplArray(a)				=> cpl_array.push(&CplVar::inew(CplDataType::CplArray(a.clone()),var.interner)),
				CplDataType::CplUninitialized(_) 		=> cpl_array.push(&CplVar::inew(CplDataType::CplUninitialized(CplUninitialized::new()),var.interner)),
				CplDataType::CplUndefined(_) 			=> cpl_array.push(&undefined!()),
				CplDataType::CplDict(d) 				=> cpl_array.push(&CplVar::inew(CplDataType::CplDict(d.clone()),var.interner)),
				_ 										=> abend!(format!("Unable to clone an array with a {} object in it", var.var)),
			};
		}

		cpl_array
	}
}

impl fmt::Display for CplArray{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if self.cpl_array.len() == 0{
			write!(f,"Empty")
		}else{	
			let mut dump_string = String::new();
			let mut begin = true;
			for var in &self.cpl_array {
				if begin{
					dump_string.push_str (&format!("{}",var));
					begin = false;
				}else{
					dump_string.push_str (&format!(",{}",var));
				}
			}
			write!(f,"{}",dump_string)
		}
	}
}


#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct CplString{
	pub cpl_string : String,
}

impl CplString{
	pub fn new(s : String) -> CplString{
		CplString{
			cpl_string : s,
		}
	}

	pub fn copy(&self) -> CplString{
		CplString::new(self.cpl_string.clone())
	}

	pub fn stringify(&mut self, scalar : f64) -> String{
		scalar.to_string()
	}

	pub fn update_operator(&mut self, _rvalue : &CplVar, _op : Opcode){
		eprintln!("    CplString.update_operator CplString.update_operator");
	}

	pub fn update(&mut self, num : &CplVar){
		if let CplDataType::CplString(ref s) = num.var{
			self.cpl_string = s.cpl_string.clone();
		}else{
			abend!(format!("from CplString.update: Update Value is not a string.  It is a {}", num.var));
		}
	}

	pub fn len(&self) -> usize{
		self.cpl_string.len()
	}

	pub fn append(&mut self, value_to_append : &CplVar){
		match &value_to_append.var{
			CplDataType::CplString(s) => {
				self.cpl_string.push_str(&s.cpl_string);
			}
			CplDataType::CplNumber(n) => {
				let s = self.stringify(n.cpl_number);
				self.cpl_string.push_str(&s);
			}
			CplDataType::CplBool(b) => {
				if b.cpl_bool {
					self.cpl_string.push_str("true");
				}else{
					self.cpl_string.push_str("false");
				}
			}

			_ => panic!("from CplString.append: can't append {}",value_to_append),
		}
	}

	fn apply_string_operator_to_string(&mut self, rvalue : &CplVar, op : Opcode){
		//	first, get the new value as as tring
		let mut newv = String::new();
		
		if let CplDataType::CplNumber(n) = &rvalue.var{
			newv.push_str(&n.cpl_number.to_string());
		} else if let CplDataType::CplString(s) = &rvalue.var{
			newv.push_str(&s.cpl_string);
		} else if let CplDataType::CplBool(b) = &rvalue.var{
			newv.push_str(&b.cpl_bool.to_string())
		} else{
			abend!(format!("from CplString.apply_binary_operator_to_string: the operand is wrong: {}", rvalue.var));
		}

		//	now perform the operation specified by op
		match op{
			Opcode::AddEq | Opcode::AppendEq => self.cpl_string.push_str(&newv),
			Opcode::Update => self.cpl_string = newv,
			_=> abend!(format!("from CplNumber.apply_operator_to_number: Unable to perform {} {} {}", self.cpl_string, op, newv)),
		}
	}

	fn apply_string_to_string(&mut self, rvalue : &str, op : Opcode){
		//	The rvalue is a string.  Try to convert it to a number.  If it converts
		//	do apply number to string otherwise it must be addEq or Update
		match rvalue.parse::<f64>(){
			Err(_) => {
				match op{
					Opcode::AddEq => self.cpl_string.push_str(rvalue),
					Opcode::Update => self.cpl_string = rvalue.to_string(),
					_=> panic!("from CplString.apply_string_to_string: may only append or replace a string with a string. got:{}",op),
				}
			}
			Ok(numeric_rvalue) => self.apply_number_to_string(numeric_rvalue, op),
		}
	}


	fn apply_number_to_string(&mut self, rvalue : f64, op : Opcode){
		//	The rvalue is a number.  Try converting THIS string to a number and if it
		//	converts apply the number via the opcode and save the result as a string
		//	if THIS string doesn't convert then append the rvalue (as a string) to
		//	THIS string
		match self.cpl_string.parse::<f64>(){
			Err(_) => {
				match op {
					Opcode::AddEq => self.cpl_string.push_str(&rvalue.to_string()),
					Opcode::Update => self.cpl_string = rvalue.to_string(),
					_=> panic!("from CplString.apply_string_to_string: only operation allowed in this congext is '+='. Got {}",op),
				}
			}
			Ok(this_n) =>{
				let mut updated_value : f64 = this_n;
				match op{
					Opcode::Update => updated_value  = this_n,
					Opcode::AddEq  => updated_value += this_n,
					Opcode::SubEq  => updated_value -= this_n,
					Opcode::DivEq  => updated_value /= this_n,
					Opcode::MulEq  => updated_value *= this_n,
					Opcode::ModEq  => updated_value = (updated_value as i32 % this_n as i32) as f64,
					Opcode::OrEq   => updated_value = (updated_value as i32 | this_n as i32) as f64,
					Opcode::AndEq  => updated_value = (updated_value as i32 & this_n as i32) as f64,
					_=> abend!(format!("from CplString.apply_string_to_string: Unable to operate to perform {} {} {}", self.cpl_string, op, this_n)),
				}
				self.cpl_string = updated_value.to_string();
			}
		}	
	}


	pub fn apply_binary_operator_to_string (&mut self, rvalue : &CplVar, op : Opcode){

		//	first, if the operator is append or equals we can do the append or the update
		//	relatively simply.  The exception is AddEq which can mean either += or . depending on
		//	the type of data
		match op{
			Opcode::Append | Opcode::AppendEq | Opcode::Update => self.apply_string_operator_to_string(rvalue, op),
			_ => {},
		}

		//	if the new value is a string and THIS string looks like a number
		//	try converting the new value to a number, applying the operation, and saving
		//	the result as a string.
		match rvalue.var{
			CplDataType::CplString(ref s) => self.apply_string_to_string(&s.cpl_string, op),
			CplDataType::CplNumber(ref n) => self.apply_number_to_string(n.cpl_number, op),
			_ => panic!("from CplString.apply_binary_operator_to_string -- Rvalue must be either string or number.  it's {}",rvalue.var)
		}
	}
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct CplBool{
	pub cpl_bool : bool,
}

impl CplBool{
	pub fn new(b : bool) -> CplBool{
		CplBool{
			cpl_bool : b,
		}
	}

	pub fn copy(&self) -> CplBool{
		CplBool::new(self.cpl_bool)
	}

	pub fn as_int(&self) -> i32{
		if self.cpl_bool{
			return 1;
		}else{
			return 0;
		}
	}

	pub fn update_operator(&mut self, _rvalue : &CplVar, _op : Opcode){
		eprintln!("    CplBool.update_operator CplBool.update_operator");
	}

	pub fn apply_daminit(&mut self){
		self.cpl_bool = !self.cpl_bool;
	}
}

pub struct CplNumber {
	pub rust_data_type : RustDataType,
	pub cpl_number : f64,
}

impl CplNumber{
	pub fn new(rust_data_type : RustDataType, data : f64) -> CplNumber{
		CplNumber{
			rust_data_type : rust_data_type,
			cpl_number : data,
		}
	}

	pub fn as_real(&self) -> f64{
		self.cpl_number
	}

	pub fn as_int(&self) ->i64{
		self.cpl_number as i64
	}

	pub fn as_char(&self) -> char{
		if self.cpl_number > 255.0{
			abend!(format!("Can't convert data to char because data > 255"));
		}

		self.cpl_number as u8 as char
	}

	pub fn as_bool(&self) -> bool{
		if self.cpl_number != 0.0{
			true
		}else{
			false
		}
	}

	pub fn apply_unary_operator (&mut self, op : Opcode){
		match op{
			Opcode::Inc	   => self.cpl_number += 1.0,
			Opcode::Dec    => self.cpl_number -= 1.0,
			Opcode::Uminus => self.cpl_number *= -1.0,
			_=> abend!(format!("from CplNumber.apply_unary_operator: Unable to apply {} to {}", op, self.cpl_number)),
		}
	}

	pub fn apply_binary_operator_to_number (&mut self, rvalue : &CplVar, op : Opcode, ){
		//	first, get the new value as just a number
		let newv : f64;
		
		if let CplDataType::CplNumber(n) = &rvalue.var{
			newv = n.cpl_number;
		}else{
			panic!("from CplNumber.update_operator: the operand is not numeric: {}", rvalue.var);
		}
		//	now perform the operation specified by op
		match op{
			Opcode::Update => self.cpl_number  = newv,
			Opcode::AddEq  => self.cpl_number += newv,
			Opcode::SubEq  => self.cpl_number -= newv,
			Opcode::DivEq  => self.cpl_number /= newv,
			Opcode::MulEq  => self.cpl_number *= newv,
			Opcode::ModEq  => self.cpl_number = (self.cpl_number as i32 % newv as i32) as f64,
			Opcode::OrEq   => self.cpl_number = (self.cpl_number as i32 | newv as i32) as f64,
			Opcode::AndEq  => self.cpl_number = (self.cpl_number as i32 & newv as i32) as f64,
			_=> abend!(format!("from CplNumber.apply_operator_to_number: Unable to operate to perform {} {} {}", self.cpl_number, op, newv)),
		}
	}
	
	pub fn add (&mut self, num : f64){
		self.cpl_number += num;
	}

	pub fn mul (&mut self, num : f64){
		self.cpl_number *= num;
	}

	pub fn div (&mut self, num : f64){
		self.cpl_number /= num;
	}

	pub fn modulo (&mut self, num : f64){
		self.cpl_number = self.cpl_number % num;
	}

	pub fn bw_and (&mut self, num : f64){
		self.cpl_number = (self.cpl_number as i64 & num as i64) as f64;
	}

	pub fn bw_or (&mut self, num : f64){
		self.cpl_number = (self.cpl_number as i64 | num as i64) as f64;
	}

	pub fn update(&mut self, num : &CplVar){
		if let CplDataType::CplNumber(ref n) = num.var{
			self.cpl_number = n.cpl_number;
		}else{
			abend!(format!("from CplNumber.update: Update Value is not a number.  It is a {}", num.var));
		}
	}

	pub fn copy(&self) -> CplNumber{
		CplNumber::new(self.rust_data_type, self.cpl_number)
	}
}
impl PartialEq for CplNumber{
	fn eq(&self, other : &Self) -> bool{
		return self.cpl_number == other.cpl_number;
	}
}
impl Eq for CplNumber{}

impl PartialOrd for CplNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		if self.cpl_number > other.cpl_number{
			return Some(Ordering::Greater);
		}else if self.cpl_number < other.cpl_number{
			return Some(Ordering::Less);
		}else{
			return Some(Ordering::Equal);
		}
    }
}
impl Ord for CplNumber {
    fn cmp(&self, other: &Self) -> Ordering {
		if self.cpl_number > other.cpl_number{
			return Ordering::Greater;
		}else if self.cpl_number < other.cpl_number{
			return Ordering::Less;
		}else{
			return Ordering::Equal;
		}
    }
}

impl fmt::Display for CplNumber{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}",self.cpl_number)
	}
}

//#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct CplFileReader{
	pub file_name : String,
	pub eof_flag : bool,
	pub last_error : String,
	pub count : usize,
	pub file_len : usize,
	pub reader : BufReader<File>,
	pub open_mode : String,
}

impl CplFileReader{
	pub fn new(file_name : &str, open_mode : &str) -> CplFileReader{
		let mut rdr = CplFileReader{
			file_name : file_name.to_string(),
			eof_flag : false,
			last_error : String::new(),
			reader : BufReader::new(File::open(file_name).expect(&format!("--Unable to open {}--\n",file_name))),
			count : 0,
			file_len : 0,
			open_mode : open_mode.to_string(),
		};


		let meta = metadata(file_name).unwrap();
		rdr.file_len = meta.len() as usize;

		rdr
	}

	//	Read one line from the file and return how many
	//	characters were read (0 = eof)
	pub fn readln(&mut self, line : & mut String) -> usize{
		self.count = self.reader.read_line(line).unwrap();
		if self.count == 0{
			self.eof_flag = true;
		}
		//self.eof_flag = self.count == 0;
		self.count
	}

	//	Read an entire file of text lines into an array
	//	The array reference is the parameter
	pub fn read(&mut self, lines_ref: &mut CplArray){
		lines_ref.clear();

		loop{
			//	get the next line of text and return if hit eof
			let mut line = String::new();
			self.count = self.reader.read_line(&mut line).unwrap();
			if self.count == 0{
				return;
			}

			//	add the line we just read to the array
			lines_ref.push(&CplVar::new(CplDataType::CplString(CplString::new(line.trim().to_string()))));
		}
	}


	pub fn read_csv(&mut self) -> CplArray{
		let mut line = String::new();
		self.count = self.reader.read_line(&mut line).unwrap();
		if self.count == 0{
			self.eof_flag = true;
			return CplArray::new();
		}
		let trimmed_line = line.trim();

		//-----------------------------------------------------------------------------
		//	TODO:	Really big stupid change.  Tokenizer should be able to parse
		//	CSV but it is too tightly integrated with the command line interpreter
		//	module (CLI). We need to disentangle it from CLI so that it can be
		//	used with either the CPL program or any other file we want to point
		//	it at -- like this CSV file for example.  For now, we'll have to do
		//	a simple CSV parser which will, replicate some of the functionality of
		//	tokenizer.  Damn!!!
		//-----------------------------------------------------------------------------

		//	now we have a line of csv text.  Parse it and put each of the elements
		//	into the return array
		let mut rtn = CplArray::new();

		let mut csv = CsvParser::new(&trimmed_line, self.open_mode.as_bytes()[1] as char);
		let mut element = String::new();
		loop{
			let ex = csv.next_element(&mut element);

			//	Now, determine if the element is a number or not.  If it is (i.e. the string to number parse
			//	was successful), then push it as a number.  Otherwise, push it as a string.
			//
			//	This means that when an appliction is processing data from a CSV it might try to do
			//	stuff that would work on a string but not on a number.  Most things down stream from
			//	here will work with either except for those things that assume the value being worked on
			//	is a string.  For example, string built-in functions.  We need to "harden" these functions
			//	in case they are handed a number.
			//
			//	For built-in functions, the "as_string()" function should always return a string, even if the
			//	operand is a number.  Really??????
			match element.parse::<f64>() {
				Err(_) => rtn.push(&CplVar::new(CplDataType::CplString(CplString::new(element.trim_end_matches('\n').to_string())))),
				Ok(parsed) => rtn.push(&CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Real, parsed)))),
			}
			if !ex{break}
		}
		return rtn;
	}
	
}
impl PartialEq for CplFileReader{
	fn eq(&self, _other : &Self) -> bool{
		abend!(format!(".....CplFileReader"));
	}
}
impl Eq for CplFileReader{}
impl PartialOrd for CplFileReader {
    fn partial_cmp(&self, _other: &Self) -> Option<Ordering> {
        abend!(format!("......CplFileReader"));
    }
}
impl Ord for CplFileReader {
    fn cmp(&self, _other: &Self) -> Ordering {
        abend!(format!("......CplFileReader"));
    }
}

pub struct CplFileWriter{
	pub file_name : String,
	pub last_error : String,
	pub writer : BufWriter<File>,
}

impl CplFileWriter{
	pub fn new(file_name : &str) -> CplFileWriter{
		CplFileWriter{
			file_name : file_name.to_string(),
			last_error : String::new(),
			writer : BufWriter::new(File::create(file_name).expect(&format!("--Unable to open {}--\n",file_name))),
		}
	}
	
	pub fn write_array(&mut self, array : &CplVar, writeln : bool){
		if let CplDataType::CplArray(ref a) = array.var{
			let mut i = 0;
			for item in &a.cpl_array{
				if i > 0{
					self.write(",",false);
				}

				match &item.var{
					CplDataType::CplNumber (n) => self.write(&n.cpl_number.to_string(),false),
					CplDataType::CplString (s) => {
						match s.cpl_string.find(','){
							None => self.write(&s.cpl_string,false),
							Some(_) => {
								self.write("\"", false);
								self.write(&s.cpl_string,false);
								self.write("\"", false);
							},
						}
						
					},
					CplDataType::CplBool   (b) => self.write(&b.cpl_bool.to_string(),false),
					_=> self.write(&format!("blat! {}",item.var),false),
				}
				i+=1;
			}
			
			if writeln{
				self.write("\n",false);
			}
		}
	}

	pub fn write(&mut self, line : &str, writeln : bool){
		if writeln{
			self.writer.write_all(format!("{}\n",line).as_bytes()).expect("Unable to write data");
		}else{
			self.writer.write_all(line.as_bytes()).expect("Unable to write data");
		}
	}
}

impl PartialEq for CplFileWriter{
	fn eq(&self, _other : &Self) -> bool{
		abend!(format!(".....CplFileWriter"));
	}
}
impl Eq for CplFileWriter{}
impl PartialOrd for CplFileWriter {
    fn partial_cmp(&self, _other: &Self) -> Option<Ordering> {
        abend!(format!("......CplFileWriter"));
    }
}
impl Ord for CplFileWriter {
    fn cmp(&self, _other: &Self) -> Ordering {
        abend!(format!("......CplFileWriter"));
    }
}

pub struct CplFileAppender{
	pub file_name : String,
	pub last_error : String,
	pub writer : BufWriter<File>,
}

impl CplFileAppender{
	pub fn new(file_name : &str) -> CplFileAppender{
		CplFileAppender{
			file_name : file_name.to_string(),
			last_error : String::new(),
			writer : BufWriter::new(OpenOptions::new()
				.append(true)
            	.create(true)
            	.open(file_name)
				.expect(&format!("--Unable to open {}--\n",file_name))),
		}
	}

	pub fn write_array(&mut self, array : &CplVar, writeln : bool){
		if let CplDataType::CplArray(ref a) = array.var{
			let mut i = 0;
			for item in &a.cpl_array{
				if i > 0{
					self.write(",",false);
				}

				match &item.var{
					CplDataType::CplNumber (n) => self.write(&n.cpl_number.to_string(),false),
					CplDataType::CplString (s) => {
						match s.cpl_string.find(','){
							None => self.write(&s.cpl_string,false),
							Some(_) => {
								self.write("\"", false);
								self.write(&s.cpl_string,false);
								self.write("\"", false);
							},
						}
						
					},
					CplDataType::CplBool   (b) => self.write(&b.cpl_bool.to_string(),false),
					_=> self.write(&format!("blat! {}",item.var),false),
				}
				i+=1;
			}
			
			if writeln{
				self.write("\n",false);
			}
		}
	}
	
	pub fn write(&mut self, line : &str, writeln : bool){
		if writeln{
			self.writer.write_all(format!("{}\n",line).as_bytes()).expect("Unable to write data");
		}else{
			self.writer.write_all(line.as_bytes()).expect("Unable to write data");
		}
	}
}
impl PartialEq for CplFileAppender{
	fn eq(&self, _other : &Self) -> bool{
		abend!(format!(".....CplFileAppender"));
	}
}
impl Eq for CplFileAppender{}
impl PartialOrd for CplFileAppender {
    fn partial_cmp(&self, _other: &Self) -> Option<Ordering> {
        abend!(format!("......CplFileAppender"));
    }
}
impl Ord for CplFileAppender {
    fn cmp(&self, _other: &Self) -> Ordering {
        abend!(format!("......CplFileAppender"));
    }
}

#[derive(PartialEq,Clone, Eq, Hash, Debug, Ord, PartialOrd)]
pub struct CplKey{
	pub key : String,
}

impl CplKey{
	pub fn new(key : &str) -> CplKey{
		CplKey {
			key : key.to_string(),
		}
	}

	//	Build a key from supported Data Types
	pub fn to_key(cpl_var : &CplDataType) -> CplKey{
		match &cpl_var{
			CplDataType::CplNumber(n) 				=> CplKey::new(&n.cpl_number.to_string()),
			CplDataType::CplString(ref s)			=> CplKey::new(&s.cpl_string),
			CplDataType::CplBool(b)					=> CplKey::new(&b.cpl_bool.to_string()),
			_										=> abend!(format!("Sorry, you can't use {} as a key to a dictionary", cpl_var)),
		}
	}
}

#[derive(Clone, PartialEq, Eq)]
pub struct CplDict{
	pub cpl_dict : HashMap<CplKey, CplVar>,
}

impl CplDict{
	pub fn new() -> CplDict{
		CplDict{
			cpl_dict : HashMap::new(),
		}
	}

	pub fn contains_key(&self, key : &CplKey) -> bool{
		return self.cpl_dict.contains_key(key);
	}

	pub fn get(&self, key : &CplKey) -> CplVar{
		if !self.contains_key(key){
			return undefined!();
		}
		return self.cpl_dict.get(key).unwrap().clone();
	}

	pub fn fetch_indexed(&self, index : &CplVar) -> CplVar{
		let key = CplKey::to_key(&index.var);
		if self.cpl_dict.contains_key(&key){
			return self.get(&key);
		}else{
			return undefined!();
		}
	}


	pub fn keys(&mut self) -> CplVar{
		let mut rtn = CplVar::new(CplDataType::CplArray(CplArray::new()));
		if let CplDataType::CplArray(ref mut a) = rtn.var{
			for cpl_key in self.cpl_dict.keys() {
				a.push(&CplVar::new(CplDataType::CplString(CplString::new(cpl_key.key.clone()))));
			}
		}
	
		rtn.clone()
	}

	pub fn len(&self) -> usize{
		self.cpl_dict.keys().len()
	}

	pub fn update_indexed(&mut self, key : &CplVar, value : &CplVar){
		let cpl_key = CplKey::to_key(&key.var);
		if self.cpl_dict.contains_key(&cpl_key){
			self.cpl_dict.insert(cpl_key,value.clone());
		}else{
			abend!(format!("from CplDict.update_indexed: Key not found {}", key));
		}
	}


	fn update_indexed_op_number(&mut self, key : &CplKey, raw_value : f64){
		self.cpl_dict.insert(key.clone(),CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Real, raw_value))));
	}

	//	Perform an operation on an element
	pub fn update_indexed_op(&mut self, index : &CplVar, rvalue : &CplVar, op : Opcode){
		let key : CplKey = CplKey::to_key(&index.var);

		if let CplDataType::CplNumber(ref new_v) = rvalue.var{
			if let CplDataType::CplNumber(ref e) = self.cpl_dict[&key].var{
				match op{
					Opcode::AddEq => self.update_indexed_op_number(&key, e.cpl_number+new_v.cpl_number),
					Opcode::SubEq => self.update_indexed_op_number(&key, e.cpl_number-new_v.cpl_number),
					Opcode::DivEq => self.update_indexed_op_number(&key, e.cpl_number/new_v.cpl_number),
					Opcode::MulEq => self.update_indexed_op_number(&key, e.cpl_number*new_v.cpl_number),
					Opcode::ModEq => self.update_indexed_op_number(&key, (e.cpl_number as i64%new_v.cpl_number as i64) as f64),
					Opcode::OrEq  => self.update_indexed_op_number(&key, (e.cpl_number as i64|new_v.cpl_number as i64) as f64),
					Opcode::AndEq => self.update_indexed_op_number(&key, (e.cpl_number as i64&new_v.cpl_number as i64) as f64),
					_=> abend!(format!("from CplArray:update_indexed_op: Expecting an assignment operator.  Got {}", op)),
				}
			}else{
				abend!(format!("from CplArray:update_indexed_op: Op {} only works on numbers. Array element is {}", op, index));
			}
		}
	}

	//	This seems to be purpose built for the builtin insert function
	pub fn insert_builtin(&mut self, key : &CplVar, value : &CplVar, update_flag : &CplVar) -> CplVar{
		let update : bool;
		if let CplDataType::CplBool(ref b) = update_flag.var{
			update = b.cpl_bool;			
		}else{
			panic! ("Expecting argument 3 to be a boolean.  Found: {}", update_flag.var);
		}

		let cpl_key = CplKey::to_key(&key.var);

		match self.cpl_dict.insert(cpl_key, value.clone()){
			None => return CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Int, self.cpl_dict.len() as f64))),
			Some(_) => if update {return CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Int, self.cpl_dict.len() as f64)))}
		}
		abend!(format!("Attempting to insert a value with an exsisting key and update_flag was false: key={}", key.var));
	}

	pub fn delete(&mut self, key : &CplVar) -> CplVar{
		let cpl_key = CplKey::to_key(&key.var);
		self.cpl_dict.remove(&cpl_key);
		CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Int, self.cpl_dict.len() as f64)))	
	}

	pub fn contains(&mut self, key : &CplVar) -> CplVar{
		let cpl_key = CplKey::to_key(&key.var);
		CplVar::new(CplDataType::CplBool(CplBool::new(self.cpl_dict.contains_key(&cpl_key))))	
	}

	pub fn push(&mut self, _item : CplVar){
		abend!(format!("Push is not a valid operation on dictionaries"));
	}

	pub fn pop(&mut self){
		abend!(format!("Pop is not a valid operation on dictionaries"));
	}

	pub fn append(&mut self, appendee : &CplVar){
		match appendee.var{
			CplDataType::CplDict(_) => {},
			_=> abend!(format!("Only dictionarys can be 'append'ed to dictionaries.  Found:{}", appendee.var)),
		}

		let mut source = appendee.clone();

		if let CplDataType::CplDict(ref mut a) = source.var{
			for key in a.cpl_dict.keys(){
				self.cpl_dict.insert(key.clone(), a.get(key));
			}
		}
	}
}

// impl PartialEq for CplDict{
// 	fn eq(&self, other : &Self) -> bool{
// 		abend!(format!(".....CplDict"));
// 	}
// }
// impl Eq for CplDict{}
impl PartialOrd for CplDict {
    fn partial_cmp(&self, _other: &Self) -> Option<Ordering> {
        abend!(format!("......CplDict"));
    }
}
impl Ord for CplDict {
    fn cmp(&self, _other: &Self) -> Ordering {
        abend!(format!("......CplDict"));
    }
}

impl fmt::Display for CplDict{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let mut dump_string = String::new();
		let mut key_list : Vec<CplKey> = Vec::new();
		for cpl_key in self.cpl_dict.keys() {
			key_list.push (cpl_key.clone());
		}
		key_list.sort();
		for key in &key_list{
			dump_string.push_str(&format!("{}={}\n", key.key, self.cpl_dict.get(key).unwrap()));
		}
		write!(f,"{}",dump_string)
	}
}



