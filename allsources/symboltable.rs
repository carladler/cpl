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

use std::cell::RefCell;
use names::*;



/****************************************
****	EntryType
*****************************************/
//	A symbol table entry can be one of many different types (or, at least
//	two at the moment).  One is for normal variables which will be used to
//	allocate space in the operand stack.  One is for struct members: these
//	are not used to allocate space.  They are used to a) link the memer to
//	the original structure definition; and b) to linke the member to its
//	location in the array created to hold members.
#[derive(Debug,Clone)]
pub enum SymbolTableEntryType{
	NormalSymbolEntry(NormalSymbolEntry),
	StructMemberEntry(StructMemberEntry),
	StructEntry(StructEntry),
	StructChildEntry(StructChildEntry),
	LiteralEntry(LiteralEntry),
}
impl fmt::Display for SymbolTableEntryType{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self{
			SymbolTableEntryType::NormalSymbolEntry(_) => write!(f,"NormalSymbolEntry"),
			SymbolTableEntryType::StructMemberEntry(_) => write!(f,"StructMemberEntry"),
			SymbolTableEntryType::StructEntry(_) => write!(f,"StructEntry"),
			SymbolTableEntryType::StructChildEntry(_) => write!(f,"StructChildEntry"),
			SymbolTableEntryType::LiteralEntry(_) => write!(f,"LiteralEntry"),
		}
	}
}



#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct NormalSymbolEntry{
	pub block_num : usize,
	pub index : usize,
	pub interner : usize,
}

impl NormalSymbolEntry{
	//	used to create an empty detail structure
	pub fn new(block_num : usize, index : usize, interner : usize) -> NormalSymbolEntry{
		NormalSymbolEntry{
			block_num : block_num,
			index : index,
			interner : interner,
		}
	}	
	//	used to create an empty detail structure
	pub fn initialize() -> NormalSymbolEntry{
		NormalSymbolEntry{
			block_num : 0,
			index : 0,
			interner : 0,
		}
	}	
}

impl fmt::Display for NormalSymbolEntry{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "block: {} index:{}", self.block_num, self.index)
	}
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct StructEntry{
	pub block_num : usize,
	pub index : usize,
	pub interner : usize,
}

impl StructEntry{
	//	used to create an empty detail structure
	pub fn new(block_num : usize, index : usize, interner : usize) -> StructEntry{
		StructEntry{
			block_num : block_num,
			index : index,
			interner : interner,
		}
	}	
	//	used to create an empty detail structure
	pub fn initialize() -> StructEntry{
		StructEntry{
			block_num : 0,
			index : 0,
			interner : 0,
		}
	}	
}

impl fmt::Display for StructEntry{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "block: {} index:{}", self.block_num, self.index)
	}
}


#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct StructMemberEntry{
	pub struct_number : usize,

	//	In order to find a member of a struct for reading or updating we
	//	have to follow this list.  It starts with the index into the underlying
	//	array and then each sub-struct following.
	pub index_list : Vec<usize>,
	pub interner : usize,
}

impl StructMemberEntry{
	//	used to create an empty detail structure
	pub fn new(struct_number : usize, index_list : &Vec<usize>, interner : usize) -> StructMemberEntry{
		StructMemberEntry{
			struct_number : struct_number,
			index_list : index_list.clone(),
			interner : interner,
		}
	}	
	//	used to create an empty detail structure
	pub fn initialize() -> StructMemberEntry{
		StructMemberEntry{
			struct_number : 0,
			index_list: Vec::new(),
			interner : 0,
		}
	}	
}

impl fmt::Display for StructMemberEntry{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "struct number: {} index: {:?} interner: {}", self.struct_number, self.index_list, self.interner)
	}
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct StructChildEntry{
	id : usize,
	parent_id : usize,
	member_number : usize,
	interner : usize
}

impl StructChildEntry{
	pub fn new(id : usize, parent_id : usize, member_number : usize, interner : usize) -> StructChildEntry{
		StructChildEntry{
			id : id,
			parent_id : parent_id,
			member_number : member_number,
			interner : interner,
		}
	}
}
impl fmt::Display for StructChildEntry{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "id: {} parent: {} member number: {} interner: {}", self.id, self.parent_id, self.member_number, self.interner)
	}
}

//	*********************************************************************
//	******  Literals
//	*********************************************************************

//	A literal is declared as:
//
//		literal <value>
//
//	where <value> can be:
//
//		LiteralString ::= "<any text>"
//		LiteralNumber ::= <any valid number>
//		LiteralBool	::= true | false
//
//		I don't know if these will work, but will include them here for
//		completeness
//
//		LiteralArray ::= <array literal>
//		LiteralDict ::= <dictionary literal>

#[derive(Debug,Clone)]
pub enum LiteralType{
	LiteralString(LiteralString),
	LiteralNumber(LiteralNumber),
	LiteralBool(LiteralBool),
	LiteralArray(LiteralArray),
	LiteralDict(LiteralDict),
}

impl fmt::Display for LiteralType{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self{
			LiteralType::LiteralString(v)		=> write!(f,"LiteralString: {}",v.literal_string),
			LiteralType::LiteralNumber(v)		=> write!(f,"LiteralNumber: {}",v.literal_number),
			LiteralType::LiteralBool(v)			=> write!(f,"LiteralBool: {}",v.literal_bool),
			LiteralType::LiteralArray(_)		=> write!(f,"LiteralArray"),
			LiteralType::LiteralDict(_)			=> write!(f,"LiteralDict"),
		}
	}
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub struct LiteralString{
	pub literal_string : String,
}

impl LiteralString{
	pub fn new(literal_string : &str) -> LiteralString{
		LiteralString{
			literal_string : literal_string.to_string(),
		}
	}
}


#[derive(Debug,Clone)]
pub struct LiteralNumber{
	pub literal_number : f64,
}

impl LiteralNumber{
	pub fn new(literal_number : f64) -> LiteralNumber{
		LiteralNumber{
			literal_number : literal_number,
		}
	}
}

impl PartialEq for LiteralNumber{
	fn eq (&self, _n:&LiteralNumber) -> bool{
		false
	}
}


#[derive(Debug,Clone)]
pub struct LiteralBool{
	pub literal_bool: bool,
}

impl LiteralBool{
	pub fn new(literal_bool : bool) -> LiteralBool{
		LiteralBool{
			literal_bool : literal_bool,
		}
	}
}



#[derive(Debug,Clone)]
pub struct LiteralArray{
	pub liberal_array : Vec<LiteralType>,
}
#[derive(Debug,Clone)]
pub struct LiteralDict{
	pub literal_dict : HashMap<String,LiteralType>,
}

#[derive(Debug,Clone)]
pub struct LiteralEntry{
	pub literal_type : LiteralType,
}

impl LiteralEntry{
	pub fn new(literal_type : LiteralType) -> LiteralEntry{
		LiteralEntry{
			literal_type : literal_type,
		}
	}
}

impl fmt::Display for LiteralEntry{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "LiteralEntry")
	}
}




/****************************************
****	Symbol Table
*****************************************/

struct GlobalSymbols{
	globals : HashMap<String, SymbolTableEntryType>,
}

impl GlobalSymbols{
	fn new() -> GlobalSymbols{
		GlobalSymbols{
			globals : HashMap::new(),
		}
	}
}

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
					SymbolTableEntryType::NormalSymbolEntry(n) => eprintln!("Normal-\"{}\" {}",symbol, n),
					SymbolTableEntryType::StructMemberEntry(m) => eprintln!("StructMemberEntry-\"{}\" {}",symbol, m),
					SymbolTableEntryType::StructEntry(s) => eprintln!("StructEntry-\"{}\" {}",symbol,s),
					SymbolTableEntryType::StructChildEntry(s) => eprintln!("StructChildEntry-\"{}\" {}",symbol,s),
					SymbolTableEntryType::LiteralEntry(l) => eprintln!("LiteralEntry-\"{}\" {}",symbol,l),
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
						SymbolTableEntryType::StructChildEntry(_) => println!("======= {} is StructChild",symbol),
						SymbolTableEntryType::LiteralEntry(_) => println!("======= {} is Literal",symbol),
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
	pub fn get_symbol_entry_from_frame(&self, symbol : &String) -> Option<SymbolTableEntryType>{
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
		match self.get_symbol_entry_from_frame(symbol){
			None => abend!(format!("from get_struct_member_entry: {} Not found in Symbol Table", symbol)),
			Some(entry) => match entry{
				SymbolTableEntryType::StructMemberEntry(sm) => return sm,
				_=> abend!(format!("from get_struct_member_entry: {} is not a struct member", symbol)),
			}
		}
	}

	//	This will return only a normal symbol if there is one.  If not, crash.
	pub fn get_normal_symbol_entry(&self, symbol : &String) -> NormalSymbolEntry{
		match self.get_symbol_entry_from_frame(symbol){
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
	pub fn add_struct_symbol(&mut self, symbol : &String, interner : usize) -> StructEntry {
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

		let struct_entry = StructEntry::new(block_num, index, interner);

		let entry = SymbolTableEntryType::StructEntry(struct_entry.clone());

		//	Add the new symbol
		symbol_table_block.table.insert(symbol.clone(), entry.clone());


		//	Update the index
		symbol_table_block.current_index = index + 1;

		//if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("SymbolTable:add_symbol \"{}\" block={} address={}", symbol.clone(), block_num, symbol_table_block.current_index);}

		struct_entry
	}

	//	Add a literal
	pub fn add_literal (&mut self, symbol : &str, value : &LiteralType){
		let block_num = self.table.len() - 1;
		let symbol_table_block = self.table.get_mut(block_num).unwrap();

		if let Some(_) = symbol_table_block.table.get(symbol){
			panic!("from SymbolTable.add_literal:  duplicates are not allowed.  Symbol={}",symbol);
		}

		let entry : LiteralEntry;

		match value{
			LiteralType::LiteralNumber (ref n) => {
				entry = LiteralEntry::new(LiteralType::LiteralNumber(LiteralNumber::new(n.literal_number)));
			}
			LiteralType::LiteralString(ref s) => {
				entry = LiteralEntry::new(LiteralType::LiteralString(LiteralString::new(&s.literal_string)))
			},
			LiteralType::LiteralBool(b) => {
				entry = LiteralEntry::new(LiteralType::LiteralBool(LiteralBool::new(b.literal_bool)));
			},
			_=> panic!("from SymbolTable.add_literal:  {} is not supported", value),
		}

		symbol_table_block.table.insert(symbol.to_string(), SymbolTableEntryType::LiteralEntry(entry.clone()));
	}

	//	Add a normal symbol and return it's entry.  If it already exists, just
	//	return the normal entry
	pub fn add_normal_symbol(&mut self, symbol : &String, interner : usize) -> NormalSymbolEntry {
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

		let normal_entry = NormalSymbolEntry::new(block_num, index, interner);

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
	pub fn add_struct_member(&mut self, member_ref : String, struct_number : usize, member_index : &Vec<usize>, interner : usize) -> StructMemberEntry{
		if self.table.len() == 0{
			panic!("FROM SymbolTable.add_struct_member ({}): trying to get current block but tables is empty", line!()-1);
		}

		//	get the current block number
		let block_num = self.table.len() - 1;

		//	get the current block
		let symbol_table_block = self.table.get_mut(block_num).unwrap();


		//	If the current block already contains the symbol then it's an error
		//	because you can only have a struct:member pair once per function
		if symbol_table_block.table.contains_key(&member_ref){
			panic!("From add_struct_member: Duplicate struct member {}", member_ref);
		}

		let struct_member_entry = StructMemberEntry::new(struct_number, member_index, interner);
		let entry = SymbolTableEntryType::StructMemberEntry(struct_member_entry.clone());

		//	Add the new symbol AND note that we do not update the index because
		//	these entries in the symbol table are not used to allocate space in
		//	the operand stack.
		symbol_table_block.table.insert(member_ref.clone(), entry);
		struct_member_entry
	}

	//	Add a substruct member.
	pub fn _add_struct_substruct(&mut self, member_ref : String, parent_index : usize, member_index : usize, child_index : usize, interner : usize) -> StructChildEntry{
		if self.table.len() == 0{
			abend!(format!("FROM SymbolTable.add_struct_substruct ({}): trying to get current block but tables is empty", line!()-1));
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

		let struct_child_entry = StructChildEntry::new(parent_index, child_index, member_index, interner);
		let entry = SymbolTableEntryType::StructChildEntry(struct_child_entry.clone());

		//	Add the new symbol AND note that we do not update the index because
		//	these entries in the symbol table are not used to allocate space in
		//	the operand stack (the operand that holds this data is an array)
		symbol_table_block.table.insert(member_ref.clone(), entry);
		struct_child_entry
	}


	//  Fetch the address of a normal entry.  Panic if the symbol is a StructMemberEntry
	pub fn get_normal_address_from_frame(&self, key : &String) -> Option<NormalSymbolEntry>{

		//println!("====================== get_normal_address_from_frame key={}",key);
		//self.dump_table();

		match self.get_symbol_entry_from_frame(key){
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
		match self.get_symbol_entry_from_frame(key){
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
	globals : GlobalSymbols,
	names : & 'a RefCell<Names>,
}

impl <'a> SymbolTable <'a>{
	pub fn new(cli : & 'a CLI<'a>, names : & 'a RefCell<Names>) -> SymbolTable<'a>{
		SymbolTable{
			cli : cli,
			tables: Vec::new(),
			globals : GlobalSymbols::new(),
			names : names,
		}
	}

	//	get the text associated with an interner
	pub fn get_interner(&self, interner : usize) -> String{
		self.names.borrow().name(interner).to_string()
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

		let rtn = frame.get_symbol_entry_from_frame(symbol);
		if let None = rtn{
			let global = self.globals.globals.get(symbol);
			match global{
				None => None,
				Some(g) => Some(g.clone()),
			}
		}else{
			rtn
		}
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

		let interner = self.names.borrow_mut().add(symbol);

		//	and return the entry (or None)
		frame.add_struct_symbol(symbol, interner)
	}

	//	Add a normal symbol and return it's entry.  If it already exists, just
	//	return the normal entry
	pub fn add_normal_symbol(&mut self, symbol : &String) -> NormalSymbolEntry {
		//	get the element of the function symbol list
		let frame : &mut SymbolTableFrame = self.tables.last_mut().unwrap(); 

		let interner = self.names.borrow_mut().add(symbol);

		//	and return the entry (or None)
		frame.add_normal_symbol(symbol, interner)
	}

	//	Add a literal symbol (but we don't need it's entry here)
	pub fn add_literal (&mut self, symbol : &String, literal_value : &LiteralType){
		//	get the element of the function symbol list
		let frame : &mut SymbolTableFrame = self.tables.last_mut().unwrap(); 
		frame.add_literal(symbol, literal_value);
	}

	//	Add a struct member.  It doesn't return anything because we're using the symbol
	//	table to just remember it and it's struct number and member_index	
	pub fn add_struct_member(&mut self, member_ref : String, struct_number : usize, member_index : &Vec<usize>) -> StructMemberEntry{
		//	get the element of the function symbol list
		let frame : &mut SymbolTableFrame = self.tables.last_mut().unwrap(); 

		let interner = self.names.borrow_mut().add(&member_ref);

		frame.add_struct_member(member_ref, struct_number, member_index, interner)
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

	pub fn add_global_symbol(&mut self, symbol : &String, entry : &SymbolTableEntryType){
		self.globals.globals.insert(symbol.clone(), entry.clone());
	}
}
