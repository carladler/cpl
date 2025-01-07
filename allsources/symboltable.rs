/*
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
*/

//use std::fs::File;
//use std::env;
//use std::io::prelude::*;
//use std::io::BufReader;
use std::fmt;
// use std::collections::HashSet;
use std::collections::HashMap;
use cli::*;
use macrolib::*;


//const STRUCT_BLOCK_NUM : usize = 0;	// STRUCTS are always at block 0

/****************************************
****	EntryType
*****************************************/
//	A symbol table entry can be one of many different types (or, at least
//	two at the moment).  One is for normal variables which will be used to
//	allocate space in the operand stack.  One is for struct members: these
//	are not used to allocate space.  They are used to a) link the memer to
//	the original structure definition; and b) to linke the member to its
//	location in the array created to hold members.
#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub enum SymbolTableEntryType{
	NormalSymbolEntry(NormalSymbolEntry),
	StructMemberEntry(StructMemberEntry),
	StructEntry(StructEntry),
}
impl fmt::Display for SymbolTableEntryType{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self{
			SymbolTableEntryType::NormalSymbolEntry(_) => write!(f,"NormalSymbolEntry"),
			SymbolTableEntryType::StructMemberEntry(_) => write!(f,"StructMemberEntry"),
			SymbolTableEntryType::StructEntry(_) => write!(f,"StructEntry"),
		}
	}
}



#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct NormalSymbolEntry{
	pub block_num : usize,
	pub index : usize,
}

impl NormalSymbolEntry{
	//	used to create an empty detail structure
	pub fn new(block_num : usize, index : usize) -> NormalSymbolEntry{
		NormalSymbolEntry{
			block_num : block_num,
			index : index,
		}
	}	
	//	used to create an empty detail structure
	pub fn initialize() -> NormalSymbolEntry{
		NormalSymbolEntry{
			block_num : 0,
			index : 0,
		}
	}	
}

impl fmt::Display for NormalSymbolEntry{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "(Normal) block: {} index:{}", self.block_num, self.index)
	}
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct StructEntry{
	pub block_num : usize,
	pub index : usize,
}

impl StructEntry{
	//	used to create an empty detail structure
	pub fn new(block_num : usize, index : usize) -> StructEntry{
		StructEntry{
			block_num : block_num,
			index : index,
		}
	}	
	//	used to create an empty detail structure
	pub fn initialize() -> StructEntry{
		StructEntry{
			block_num : 0,
			index : 0,
		}
	}	
}

impl fmt::Display for StructEntry{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "(Struct) block: {} index:{}", self.block_num, self.index)
	}
}






#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct StructMemberEntry{
	pub struct_number : usize,
	pub member_index : usize,
}

impl StructMemberEntry{
	//	used to create an empty detail structure
	pub fn new(struct_number : usize, member_index : usize) -> StructMemberEntry{
		StructMemberEntry{
			struct_number : struct_number,
			member_index : member_index,
		}
	}	
	//	used to create an empty detail structure
	pub fn initialize() -> StructMemberEntry{
		StructMemberEntry{
			struct_number : 0,
			member_index: 0,
		}
	}	
}

impl fmt::Display for StructMemberEntry{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "struct number: {} index: {}", self.struct_number, self.member_index)
	}
}

/****************************************
****	Symbol Table
*****************************************/


struct SymbolTableBlock{
	// For each block, this keeps track of the current index
	current_index : usize,

	//	For each block, this maps a symbol to its block, index and address mode
	table : HashMap<String, SymbolTableEntryType>,
}

impl SymbolTableBlock{
	fn new()->SymbolTableBlock{
		SymbolTableBlock{
			current_index : 0,
			table : HashMap::new(),
		}
	}

	// fn remove_symbol(&mut self, symbol : &str){
	// 	if table.contains.key(symbol){
	// 		table.delete(symbol);
	// 		current_index -= 1;
	// 	}
	// }

	fn dump_table(&self){
		if self.table.keys().len() > 0{
			for symbol in self.table.keys(){
				match self.table.get(symbol).unwrap(){
					SymbolTableEntryType::NormalSymbolEntry(n) => println!("{}:{}",symbol, n),
					SymbolTableEntryType::StructMemberEntry(m) => println!("{}:{}",symbol, m),
					SymbolTableEntryType::StructEntry(s) => println!("{}:{}",symbol,s),
				}
			}	
		}
	}
}

//	A Frame in the symbol table is holding tank for blocks associated
//	with an active function.  We include the name of the function for diagnostic
//	purposes
struct SymbolTableFrame{
	function_name : String,
	table : Vec<SymbolTableBlock>,
}

impl SymbolTableFrame{
	fn new(function_name : String)->SymbolTableFrame{
		SymbolTableFrame{
			function_name: function_name,
			table : Vec::new(),
		}
	}

	//	add a block to the block list for this frame
	fn push_block(&mut self){
		self.table.push(SymbolTableBlock::new());
	}

	fn pop_block(&mut self){
		self.table.pop();
	}

	// fn remove_symbol(&mut self, symbol : &str){
	// 	let table = self.table.last_mut().unwrap();
	// 	table.remove_symbol(symbol);
	// }

	fn dump_table(&self){
		println!("Function: {}", self.function_name);
		for table in &self.table{
			table.dump_table();
		}
	}

	fn clear(&mut self){
		self.table.clear();
	}

	//	for debugging:  println the entry type of a symbole
	pub fn print_entry_type_in_frame (&self, symbol : &String){
		for block in self.table.iter().rev(){
			let rtn = block.table.get(symbol);
			match rtn{
				None => continue,
				Some(r) => {
					match r{
						SymbolTableEntryType::StructMemberEntry(_) => println!("======= {} is Struct Member",symbol),
						SymbolTableEntryType::NormalSymbolEntry(_) => println!("======= {} is Normal",symbol),
						SymbolTableEntryType::StructEntry(_) => println!("======= {} is Struct",symbol),
					}
					return;
				}
			}
		}

		println!("from print_entry_type_in_frame:  {} not found", symbol);
	}


	//	look for a symbol and if it's present, return the detail.  Note that it
	//	starts at the last block and moves up until it finds an entry.  This
	//	ensures that the most "local" symbol is used first.  This will return either
	//	a Normal Symbol or StructMemberEntry
	pub fn get_symbol_entry(&self, symbol : &String) -> Option<SymbolTableEntryType>{
		for block in self.table.iter().rev(){
			let rtn = block.table.get(symbol);
			match rtn{
				None => continue,
				Some(r) => return Some(r.clone()),
			}
		}
		None
	}

	pub fn get_struct_member_entry(&self, symbol : &String) -> StructMemberEntry{
		match self.get_symbol_entry(symbol){
			None => abend!(format!("from get_struct_member_entry: {} Not found in Symbol Table", symbol)),
			Some(entry) => match entry{
				SymbolTableEntryType::StructMemberEntry(sm) => return sm,
				_=> abend!(format!("from get_struct_member_entry: {} is not a struct member", symbol)),
			}
		}
	}

	//	This will return only a normal symbol if there is one.  If not, crash.
	pub fn get_normal_symbol_entry(&self, symbol : &String) -> NormalSymbolEntry{
		match self.get_symbol_entry(symbol){
			None => panic!("from get_normal_symbol_entry:  {} not found in symbol table at all", symbol),
			Some(entry) => {
				match entry {
					SymbolTableEntryType::NormalSymbolEntry(normal) => normal,
					_ => panic!("from get_normal_symbol_entry:  {} is present but normal -- expecting struct:member", symbol),
				}
			}
		}
	}

	//	Does exactly the same thing as add_normal_symbol but sets the type to
	//	StructEntry
	pub fn add_struct_symbol(&mut self, symbol : &String) -> StructEntry {
		//	get the current block number
		let block_num = self.table.len() - 1;

		//println!("{}SymbolTableFrame.add_struct_symbol: block_num: {}", DEBUG_INDENT, block_num);

		//	get the current block
		let symbol_table_block = self.table.get_mut(block_num).unwrap();


		//	If the current block already contains the symbol then
		//	we return it's detail
		match symbol_table_block.table.get(symbol){
			Some(d) => if let SymbolTableEntryType::StructEntry(n) = d{
				return n.clone();
			},
			None => {}
		};


		//	otherwise, get the current index
		let index = symbol_table_block.current_index;

		let struct_entry = StructEntry::new(block_num, index);

		let entry = SymbolTableEntryType::StructEntry(struct_entry.clone());

		//	Add the new symbol
		symbol_table_block.table.insert(symbol.clone(), entry.clone());


		//	Update the index
		symbol_table_block.current_index = index + 1;

		//if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("SymbolTable:add_symbol \"{}\" block={} address={}", symbol.clone(), block_num, symbol_table_block.current_index);}

		struct_entry
	}

	//	Add a normal symbol and return it's entry.  If it already exists, just
	//	return the normal entry
	pub fn add_normal_symbol(&mut self, symbol : &String) -> NormalSymbolEntry {
		//	get the current block number
		let block_num = self.table.len() - 1;

		//println!("{}SymbolTableFrame.add_normal_symbol: block_num: {}", DEBUG_INDENT, block_num);

		//	get the current block
		let symbol_table_block = self.table.get_mut(block_num).unwrap();


		//	If the current block already contains the symbol then
		//	we return it's detail
		match symbol_table_block.table.get(symbol){
			Some(d) => if let SymbolTableEntryType::NormalSymbolEntry(n) = d{
				return n.clone();
			},
			None => {}
		};


		//	otherwise, get the current index
		let index = symbol_table_block.current_index;

		let normal_entry = NormalSymbolEntry::new(block_num, index);

		let entry = SymbolTableEntryType::NormalSymbolEntry(normal_entry.clone());

		//	Add the new symbol
		symbol_table_block.table.insert(symbol.clone(), entry.clone());


		//	Update the index
		symbol_table_block.current_index = index + 1;

		//if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("SymbolTable:add_symbol \"{}\" block={} address={}", symbol.clone(), block_num, symbol_table_block.current_index);}

		normal_entry
	}
	//	Add a struct member.  It doesn't return anything because we're using the symbol
	//	table to just remember it and it's struct number and member_index	
	pub fn add_struct_member(&mut self, member_ref : String, struct_number : usize, member_index : usize){
		if self.table.len() == 0{
			abend!(format!("FROM SymbolTable.add_struct_member ({}): trying to get current block but tables is empty", line!()-1));
		}

		//	get the current block number
		let block_num = self.table.len() - 1;

		//	get the current block
		let symbol_table_block = self.table.get_mut(block_num).unwrap();


		//	If the current block already contains the symbol then it's an error
		//	because you can only have a struct:member pair once per function
		if symbol_table_block.table.contains_key(&member_ref){
			abend!(format!("From add_struct_member: Duplicate struct member {}", member_ref));
		}

		let entry = SymbolTableEntryType::StructMemberEntry(StructMemberEntry::new(struct_number, member_index));
	
		//	Add the new symbol AND note that we do not update the index because
		//	these entries in the symbol table are not used to allocate space in
		//	the operand stack.
		symbol_table_block.table.insert(member_ref.clone(), entry.clone());
	}

	//  Fetch the address of a normal entry.  Panic if the symbol is a StructMemberEntry
	pub fn get_normal_address_from_frame(&self, key : &String) -> Option<NormalSymbolEntry>{

		//println!("====================== get_normal_address_from_frame key={}",key);
		//self.dump_table();

		match self.get_symbol_entry(key){
			None => return None,
			Some(entry) => {
				if let SymbolTableEntryType::NormalSymbolEntry(normal) = entry{
					return Some(normal);
				}else{
					panic!("From SymbolTable.get_normal_address_from_frame: Expecting a NormalSymbolEntry symbol, {} is a StructMemberEntry", key);
				}
			}
		}
	}	
	//  Fetch the address of an instantiated struct entry.
	pub fn get_struct_address_from_frame(&self, key : &String) -> Option<StructEntry>{
		match self.get_symbol_entry(key){
			None => return None,
			Some(entry) => {
				if let SymbolTableEntryType::StructEntry(struct_entry) = entry{
					return Some(struct_entry);
				}else{
					panic!("From SymbolTable.get_struct_address_from_frame: Expecting a StructEntry symbol, {} is a {}", key, entry);
				}
			}
		}
	}	
}

pub struct SymbolTable <'a> {
	cli : & 'a CLI<'a>,

	//	The symbol table is a vector of symbol table blocks
	tables : Vec<SymbolTableFrame>,
}

impl <'a> SymbolTable <'a>{
	pub fn new(cli : & 'a CLI<'a>) -> SymbolTable{
		SymbolTable{
			cli : cli,
			tables: Vec::new(),
		}
	}

	//  Clear the symbol table.  This happens at the beginning of each function
	pub fn clear(&mut self){
		for table in &mut self.tables{
			table.clear();
		}
		self.tables.clear();
	}

	//	Add symboltable frame
	pub fn add_frame(&mut self, function_name : &str){
		self.tables.push(SymbolTableFrame::new(function_name.to_string()));
		//println!("{}SymbolTable.add_frame frame={}", DEBUG_INDENT, self.tables.len()-1);
	}

	pub fn current_frame(&self) -> usize{
		return self.tables.len()-1;
	}

	//	add an operand block to the current frame.  Each new block, has a current index
	//	(i.e. the index of each new symbol that's added and a hash table)
	pub fn push_block(&mut self){
		//	get the element of the function symbol list
		let frame : &mut SymbolTableFrame = self.tables.last_mut().unwrap(); 
		frame.push_block();
	}

	//	At the end of the current block, we pop it off the tables stack
	pub fn pop_block(&mut self){

		//	get the element of the function symbol list
		let frame : &mut SymbolTableFrame = self.tables.last_mut().unwrap(); 
		frame.pop_block();
	}

	//	In some cases we need to remove an entry from the current frame
	pub fn remove_symbol(&mut self, symbol : &str){
		let frame : &mut SymbolTableFrame = self.tables.last_mut().unwrap();
		let table = frame.table.last_mut().unwrap();
		//let block = table.table.last_mut().unwrap();
		if table.table.contains_key(symbol){
			table.table.remove(symbol);
			table.current_index -= 1;
		}
	}

	//	For debugging:  println the entry type of a symbol
	pub fn print_entry_type(&mut self, symbol : &String){
		let frame : &SymbolTableFrame = self.tables.last_mut().unwrap(); 
		frame.print_entry_type_in_frame(symbol);
	}

	//	look for a symbol and if it's present, return the detail.  Note that it
	//	starts at the last block and moves up until it finds an entry.  This
	//	ensures that the most "local" symbol is used first.  This will return either
	//	a Normal Symbol or StructMemberEntry
	pub fn get_symbol_entry(&mut self, symbol : &String) -> Option<SymbolTableEntryType>{
		//	get the element of the function symbol list
		let frame : &mut SymbolTableFrame = self.tables.last_mut().unwrap(); 

		//	and return the entry (or None)
		frame.get_symbol_entry(symbol)
	}


	pub fn get_struct_member_entry(&mut self, symbol : &String) -> StructMemberEntry{
		//	get the element of the function symbol list
		let frame : &mut SymbolTableFrame = self.tables.last_mut().unwrap(); 

		//	and return the entry (or None)
		frame.get_struct_member_entry(symbol)
	}

	//	This will return only a normal symbol if there is one.  If not, crash.
	pub fn get_normal_symbol_entry(&mut self, symbol : &String) -> NormalSymbolEntry{
		//	get the element of the function symbol list
		let frame : &mut SymbolTableFrame = self.tables.last_mut().unwrap(); 

		//	and return the entry (or None)
		frame.get_normal_symbol_entry(symbol)
	}

	//	Add a struct symbol and return it's entry.  If it already exists, just
	//	return the normal entry
	pub fn add_struct_symbol(&mut self, symbol : &String) -> StructEntry {
		//	get the element of the function symbol list
		let frame : &mut SymbolTableFrame = self.tables.last_mut().unwrap(); 

		//	and return the entry (or None)
		frame.add_struct_symbol(symbol)
	}

	//	Add a normal symbol and return it's entry.  If it already exists, just
	//	return the normal entry
	pub fn add_normal_symbol(&mut self, symbol : &String) -> NormalSymbolEntry {
		//	get the element of the function symbol list
		let frame : &mut SymbolTableFrame = self.tables.last_mut().unwrap(); 

		//	and return the entry (or None)
		frame.add_normal_symbol(symbol)
	}

	//	Add a struct member.  It doesn't return anything because we're using the symbol
	//	table to just remember it and it's struct number and member_index	
	pub fn add_struct_member(&mut self, member_ref : String, struct_number : usize, member_index : usize){
		//	get the element of the function symbol list
		let frame : &mut SymbolTableFrame = self.tables.last_mut().unwrap(); 
		frame.add_struct_member(member_ref, struct_number, member_index);
	}

	//  Fetch the address of a normal entry.  Panic if the symbol is a StructMemberEntry
	pub fn get_normal_address(&mut self, key : &String) -> NormalSymbolEntry{
		//	get the element of the function symbol list
		//	and return the entry (or None)
		match self.tables.last_mut().unwrap().get_normal_address_from_frame(key){
			None => {
				self.symbol_table_dump_diag(&format!("From SymbolTable.get_normal_address - symbol \"{}\" not found.  You must create a symbol before using it (e.g. via assignment).\n--- Symbol Table Dump ---", key));
				panic!("From SymbolTable.get_normal_address");
			}
			Some(entry) => return entry,
		}
	}

	pub fn get_struct_address(&mut self, key : &String) -> StructEntry{
		//	get the element of the function symbol list
		//	and return the entry (or None)
		match self.tables.last_mut().unwrap().get_struct_address_from_frame(key){
			None => {
				self.symbol_table_dump_diag(&format!("From SymbolTable.get_struct_address - symbol \"{}\" not found.  Evidently the struct was never instantiated.\n--- Symbol Table Dump ---", key));
				abend!(format!(""));
			},
			Some(entry) => return entry,
		}

	}

	pub fn symbol_table_dump_diag(&self, text : &str){
		println!("{}", text);
		self.symbol_table_dump_absolutely();
	}

	fn symbol_table_dump_absolutely(&self){
		for table in &self.tables{
			if table.table.len() > 0{
				table.dump_table();
			}
		}
	}

	pub fn symbol_table_dump(&self){
		if !self.cli.is_debug_bit(DUMP_SYMBOL_TABLE){return}
		self.symbol_table_dump_absolutely();
	}
}
