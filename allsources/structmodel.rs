use tokenizer::*;
use std::fmt;


/****************************************
****	Struct
*****************************************/

#[derive(Clone)]
pub enum StructMemberType{
	Field(Field),
	Substruct(Substruct),
}


#[derive(Clone)]
pub struct Struct{
	pub name : String,
	pub members : MemberList,
}

impl Struct{
	pub fn new(name : &str) -> Struct{
		Struct{
			name : name.to_string(),
			members : MemberList::new(),
		}
	}

	pub fn dump_structs(&self, header_index : usize){
		println!("struct: {} members:{} header:{}", self.name,self.members.len(), header_index);
		for member in &self.members.members{
			match member{
				StructMemberType::Field (field) => println!("    field: {}", field.name),
				StructMemberType::Substruct (child)  => println!("    child: {}",child.name),
			}
		}
	}
}

/****************************************
****	StructMemberType: Field
*****************************************/
#[derive(Clone)]
pub struct Field{
	pub name : String,
	pub initializer : Vec<Token>,
}

impl Field{
	pub fn new(name : String, initializer : &Vec<Token>) -> Field{
		Field{
			name : name,
			initializer : initializer.clone(),
		}
	}
}

impl fmt::Display for Field{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"Field: {},{}",self.name, token_list_text(&self.initializer))
	}
}

/****************************************
****	StructMemberType: Substruct
*****************************************/
#[derive(Clone)]
pub struct Substruct{
	pub name : String,
	pub id : usize,
}

impl Substruct{
	pub fn new(name : &str, id : usize) -> Substruct{
		Substruct{
			name : name.to_string(),
			id : id,
		}
	}
}

impl fmt::Display for Substruct{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"Substruct: {},{}",self.name, self.id)
	}
}


/****************************************
****	MemberList
*****************************************/
#[derive(Clone)]
pub struct MemberList{
 	pub members : Vec<StructMemberType>
}
impl MemberList{
	pub fn new() -> MemberList{
		MemberList{
			members : Vec::new(),
		}
	}

	pub fn len(&self) -> usize{
		self.members.len()
	}

	pub fn push(&mut self, new_member : &StructMemberType){
		self.members.push(new_member.clone());
	}
}

impl fmt::Display for MemberList{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let mut rslt = String::new();
		let mut start : bool = true;
		for member in &self.members{
			if !start{
				rslt.push(',');
				rslt.push(' ');
			}
			match member{
				StructMemberType::Field(field) => rslt.push_str(&field.name.clone()),
				StructMemberType::Substruct(sub) => rslt.push_str(&sub.name.clone()),
			}

			if start{
				start = false;
			}
		}
		write!(f,"{}",rslt)
	}
}
