
pub struct CsvParser{
	csv : String,
	quote : bool,
	char_pointer : usize,
	delimiter : char,
	escaped : bool,
}

impl CsvParser{
	pub fn new(csv : &str, delimiter : char) -> CsvParser{
		CsvParser{
			csv : csv.to_string(),
			quote : false,
			char_pointer : 0,
			delimiter : delimiter,
			escaped : false,
		}
	}

	//	get the next element from a CSV String.  Return true if
	//	there is one and false if we are at the end of the line
	//	if the return is true, then element contains the next
	//	element
	pub fn next_element(&mut self, element : &mut String) -> bool{
		element.clear();
		self.quote = false;

		//	loop until either end of the csv string or an element
		//	has been found
		loop{
			match self.next_char(){
				None => {
					//	This is the end of the string but the element
					//	still has what's been collected up to this point
					return false;
				},
	
				Some(c) => if self.process(c, element) {
					return true;
				},
			}	
		}
	}

	//	Get the next character from the csv string.  Return None
	//	if there isn't one
	fn next_char(&mut self) -> Option<char>{
		if self.char_pointer >= self.csv.len(){
			return None;
		}

		let c = self.csv.as_bytes()[self.char_pointer] as char;
		self.char_pointer += 1;
		Some(c)
	}


	//	This is the guts of the csv parser.  There is a bit of state
	//	machine here (but not much).  Returns true if the element has been
	//	completely found, otherwise return false
	fn process(&mut self, c : char, element : &mut String) -> bool {
		if self.escaped{
			self.escaped = false;
			return false;
		}

		if c == self.delimiter && self.quote{
			element.push(c);
			return false;
		}else if c == self.delimiter && !self.quote{
			return true;
		}

		match c{
			//	if we see a double quote and we're in a quoted string then
			//	reset the quote flag otherwise set the quote flag indicating
			//	that we can ignore ","
			'"'		=> {
				if self.quote{
					self.quote = false;
				}else{
					self.quote = true;
				}
				return false;
			},

			'\\'	=> {
				self.escaped = true;
				return false;
			}
			
			//	if anything else, then it's just part of the element
			_ 		=> {
				element.push(c);
				return false;
			}

		}
	}
}