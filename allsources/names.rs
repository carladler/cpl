//use std::fmt;
use std::collections::HashMap;


pub struct Names{
	pub names : Vec<String>,
	pub nameset : HashMap<String, usize>,
}

impl Names{
	pub fn new() -> Names{
		Names{
			names : Vec::new(),
			nameset : HashMap::new(),
		}
	}

	pub fn name(&self, internal : usize) -> &str{
		&self.names[internal]
	}

	pub fn add(&mut self, name : &str) -> usize{
		if self.nameset.contains_key(name){
			*self.nameset.get(name).unwrap()
		}else{
			self.names.push(name.to_string());
			self.nameset.insert(name.to_string(), self.names.len() - 1);
			self.names.len() - 1
		}
	}
}