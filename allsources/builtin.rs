use cplvar::*;
use regex::Regex;	
use regex::Match;
use macrolib::*;


//****************************************************************
//***************  EXTERNAL FUNCTION METADATA  *******************
//****************************************************************

#[derive(Clone)]
pub struct BuiltinFunctionDetail{
	pub target : fn(&mut BuiltinFunctions, &Vec<CplVar>, &mut OperandStack) -> CplVar,
	pub name : String,
	pub arguments : Vec<String>,
}

impl BuiltinFunctionDetail{
	pub fn new (target : fn(&mut BuiltinFunctions, &Vec<CplVar>, &mut OperandStack) -> CplVar, name : String, arguments : Vec<String>) ->BuiltinFunctionDetail{
		BuiltinFunctionDetail{
			target : target,
			name : name,
			arguments : arguments,
		}
	}
}

#[derive(Clone)]
pub struct BuiltinFunctions{
	//	Builtin Functions (or at least their proxy, accepts a vector of values
	//	and return a value)
	pub builtin_function_list : Vec<BuiltinFunctionDetail>,
	pub opens : Vec<CplVar>,
	pub compiled_regex_list : Vec<Regex>,
}

impl BuiltinFunctions {
	pub fn new() -> BuiltinFunctions{
		BuiltinFunctions{
			opens : Vec::new(),
			compiled_regex_list : Vec::new(),
			builtin_function_list : vec!
				[
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_dump_operands, "Dump".to_string(), 
						["message".to_string()].to_vec()),
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_substring, "Substr".to_string(), 
						["haystack".to_string(), "start".to_string(), "length".to_string()].to_vec()),
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_get_cpl_type, "Type".to_string(), 
						["var_to_type".to_string()].to_vec()),

					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_locate, "Locate".to_string(), 
						["haystack".to_string(), "lookfor".to_string(), "start".to_string()].to_vec()),
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_replace, "Replace".to_string(), 
						["haystack".to_string(), "needle".to_string(), "replacement".to_string(), "start".to_string()].to_vec()),
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_match, "Match".to_string(), 
						["haystack".to_string(), "needle".to_string(), "start".to_string()].to_vec()),
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_capture, "Capture".to_string(), 
						["haystack".to_string(), "needle".to_string(), "start".to_string()].to_vec()),
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_split, "Split".to_string(), 
						["haystack".to_string(), "delimiter".to_string()].to_vec()),
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_regex, "Regex".to_string(), 
						["needle".to_string()].to_vec()),

					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_length, "Length".to_string(), 
						["collection".to_string()].to_vec()),

					//	Collection Functions
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_keys, "Keys".to_string(), 
						["collection".to_string()].to_vec()),
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_sort, "Sort".to_string(), 
						["collection".to_string()].to_vec()),
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_push, "Push".to_string(), 
						["collection".to_string(),"item_to_push".to_string()].to_vec()),
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_pop, "Pop".to_string(), 
						["collection".to_string()].to_vec()),
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_insert, "Insert".to_string(), 
						["collection".to_string(),"key".to_string(),"value".to_string(), "update_flag".to_string()].to_vec()),
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_delete, "Delete".to_string(), 
						["collection".to_string(),"key".to_string()].to_vec()),
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_contains, "Contains".to_string(), 
						["collection".to_string(),"key".to_string()].to_vec()),
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_append, "Append".to_string(), 
						["collection".to_string(),"stuff_to_append".to_string()].to_vec()),

					//	File I/O Functions
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_fopen, "Fopen".to_string(), 
						["file_name".to_string(), "open_mode".to_string()].to_vec()),
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_fread, "Fread".to_string(), 
						["file_handle".to_string(), "lines".to_string()].to_vec()),
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_freadln, "Freadln".to_string(), 
						["file_handle".to_string()].to_vec()),
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_fwrite, "Fwrite".to_string(), 
						["file_handle".to_string(), "line".to_string()].to_vec()),
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_fwriteln, "Fwriteln".to_string(), 
						["file_handle".to_string(), "line".to_string()].to_vec()),
					BuiltinFunctionDetail::new(BuiltinFunctions::builtin_feof, "Feof".to_string(), 
						["file_handle".to_string()].to_vec()),
				],
		}
	}

	/*********************************************************
		Builtin Functions
	*********************************************************
		these functions are accessed via an index into the builtin
		function table.

		all function accept a reference to a stack of CplVars
		meaning that the first argument is at the end, the second
		at end - 1 and so on.

		Each of these functions edits the arguments list for length
		and data type, crashing if anything is wrong
	*********************************************************/

	pub fn builtin_dump_operands(&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		if let CplDataType::CplString(ref s) = arguments[0].var{
			operand_stack.dump_operands_with_message(&s.cpl_string);
		}
		return CplVar::new(CplDataType::CplUninitialized(CplUninitialized::new()));
	}	

	//	Returns the length of a CplVar.  If the var is not a collection the length
	//	will be 1;  If the var is an array this is the usual len() function.  I'm not
	//	sure yet what the length of a dictinary is so, for now, it is just the number
	//	entries.
	pub fn builtin_length(&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		if (arguments.len()) != 1{
			abend!(format!("from Builtin Function 'Length' - Expecing 1 argument, found {}", arguments.len()));
		}
		let len = operand_stack.len(&arguments[0]);
		CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Int, len as f64)))
	}


	//	Return a substring of a CplString var:
	//  	usage substr(haystack, start, len)
	//		where start is 0 relative
	pub fn builtin_substring(&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		if (arguments.len()) != 3{
			abend!(format!("from Builtin Function 'Substr' - Expecing 3 arguments, found {}", arguments.len()));
		}

		let argslen = arguments.len();
		let haystack = operand_stack.dereference(&arguments[argslen-1]).as_string();
		let start = operand_stack.dereference(&arguments[argslen-2]).as_number();
		let len = operand_stack.dereference(&arguments[argslen-3]).as_number();

		if (start+len) > haystack.len() as f64{
			panic!("from Builtin Function 'substr' - start + len > length of haystack");
		}
		let ss: String = haystack.chars().skip(start as usize).take(len as usize).collect();
		
		CplVar::new(CplDataType::CplString(CplString::new(ss.clone())))
	}

	fn replace_regex(&mut self, haystack : String, needle : &String, replacement : &str, start : usize, _operand_stack : &mut OperandStack) -> CplVar{
		let haystack_suffix: String = haystack.chars().skip(start as usize).collect();
		let haystack_prefix: String = haystack.chars().take(start).collect();
		
		let re = Regex::new(needle).unwrap();
		let temp = re.replace_all(&haystack_suffix, replacement);
		let updated = format!("{}{}", haystack_prefix, temp);
		CplVar::new(CplDataType::CplString(CplString::new(updated)))
	}

	//	Replace a substring in a haystack with new next.  parameters:
	//
	//		heystack:  a string containing the substring to replace
	//		needle: a string to search for OR a regular expression
	//		replacement: a string to substitute needle
	pub fn builtin_replace(&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		if arguments.len() != 4 {
			abend!(format!("from Builtin Function 'Locate' - Expecing 4 arguments, found {}", arguments.len()));
		}

		let argslen = arguments.len();
		let haystack = operand_stack.dereference(&arguments[argslen-1]).as_string();
		let needle = operand_stack.dereference(&arguments[argslen-2]).as_string();
		let replacement = operand_stack.dereference(&arguments[argslen-3]).as_string(); 
		let start = operand_stack.dereference(&arguments[argslen-4]).as_number() as usize;

		//	If we got passed a zero length haystack then just return it
		if haystack.len() == 0{
			return CplVar::new(CplDataType::CplString(CplString::new(haystack.clone())));
		}

		//	if the needle begins and ends with "/" characters then we treat it
		//	as a regular expression.
		if needle.len() > 2 && needle.as_bytes()[0] as char == '/' && needle.as_bytes()[needle.len()-1] as char == '/'{
			if needle.len() > 3 && needle.as_bytes()[needle.len()-2] as char != '\\'{
				let local_needle = needle.chars().skip(1).take(needle.len()-2).collect();
				return self.replace_regex(haystack.clone(), &local_needle, &replacement, start, operand_stack);
			}
		}

		let start_limit = haystack.len() - needle.len();
		if start > start_limit{
			// abend!(format!("from Builtin Function 'Replace' - start is > haystack length - match length"));
			panic!("from Builtin Function 'Replace' - start is > haystack length - match length: start={} haystack={} needle={} limit={}", start, haystack.len(), needle.len(), start_limit);
		}
		let rslt = haystack.get(start..).unwrap().replace(&needle, &replacement);
		
		//let ss: String = haystack.chars().skip(start as usize).take(len as usize).collect();
		let ss: String = haystack.chars().take(start as usize).collect();
		let rtn = format!("{}{}",ss,rslt);
		CplVar::new(CplDataType::CplString(CplString::new(rtn.clone())))
	}

	pub fn builtin_split(&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		if arguments.len() != 2 {
			abend!(format!("from Builtin Function 'Split' - Expecing 2 arguments, found {}", arguments.len()));
		}

		let argslen = arguments.len();
		let haystack = operand_stack.dereference(&arguments[argslen-1]).as_string();
		let delimiter = operand_stack.dereference(&arguments[argslen-2]).as_string();
		
		let rslt : Vec<String>;

		// //	if the delimiter begins and ends with "/" characters then we treat it
		// //	as a regular expression.
		if delimiter.len() > 2 && delimiter.as_bytes()[0] as char == '/' && delimiter.as_bytes()[delimiter.len()-1] as char == '/'{
			if delimiter.len() > 3 && delimiter.as_bytes()[delimiter.len()-2] as char != '\\'{
				let regex_delimiter : String = delimiter.chars().skip(1).take(delimiter.len()-2).collect();
				let re = Regex::new(&regex_delimiter).unwrap();
				rslt = re.split(&haystack).map(|m| m.to_string()).collect();
				//println!(".... {:?}",rslt);
			}else{
				rslt = haystack.split(&delimiter).map(|m| m.to_string()).collect();
			}
		}else{
			rslt = haystack.split(&delimiter).map(|m| m.to_string()).collect();
			//println!(".... {:?}",rslt);
		}


		let mut rtn_array = CplArray::new();
		for m in &rslt{
			rtn_array.push(&CplVar::new(CplDataType::CplString(CplString::new(m.to_string()))));
		}
		return CplVar::new(CplDataType::CplArray(rtn_array));		
	}	

	fn match_or_capture_help(&mut self, match_or_cap : char, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		if arguments.len() != 3 {
			abend!(format!("from Builtin Function 'Match' - Expecing 3 arguments, found {}", arguments.len()));
		}

		let argslen = arguments.len();
		let haystack = operand_stack.dereference(&arguments[argslen-1]).as_string();
		
		//	If needle is a CplNumber then it is a reference to an existing pre-compiled
		//	regex.
		let needle = operand_stack.dereference(&arguments[argslen-2]);

		let start = operand_stack.dereference(&arguments[argslen-3]).as_number() as usize;

		if start >= haystack.len() {
			abend!(format!("from Builtin Function 'Match' - start is > haystack length"));
		}

		let haystack_suffix: String = haystack.chars().skip(start as usize).collect();
		
		let re : Regex;
		if let CplDataType::CplString(regex) = needle.var{
			re = Regex::new(&regex.cpl_string).unwrap();
		}else if let CplDataType::CplNumber(n) = needle.var{
			re = self.compiled_regex_list[n.cpl_number as usize].clone();
		}else{
			if match_or_cap == 'm'{
				abend!(format!("from Builtin Function 'Match' - needle must be either regex string or the number returned from Regex)"));
			}else{
				abend!(format!("from Builtin Function 'Capture' - needle must be either regex string or the number returned from Regex)"));
			}
		}
		
		let mut rtn_array = CplArray::new();

		if match_or_cap == 'm'{
			let matches: Vec<Match> = re.find_iter(&haystack_suffix).map(|m| m).collect();

			for m in &matches{
				let item = format!("{}:{}:{}",m.start(), m.end(), m.as_str());
				rtn_array.push(&CplVar::new(CplDataType::CplString(CplString::new(item))));
			}
			return CplVar::new(CplDataType::CplArray(rtn_array));	
		}else{
			match re.captures(&haystack_suffix){
				Some(cap) => {
					let mut i=0;
					while i<cap.len(){
						rtn_array.push(&CplVar::new(CplDataType::CplString(CplString::new(cap[i].to_string()))));
						i+=1;
					}
					return CplVar::new(CplDataType::CplArray(rtn_array));
				}
				None => {
					return CplVar::new(CplDataType::CplArray(rtn_array));
				}
			}
		}
		
	}

	pub fn builtin_capture (&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		return self.match_or_capture_help('c', arguments, operand_stack);
	}


	pub fn builtin_match(&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		return self.match_or_capture_help('m', arguments, operand_stack);
	}

	//	Compile a regex and put in the compiled_regex_list
	pub fn builtin_regex(&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		if arguments.len() != 1 {
			abend!(format!("from Builtin Function 'Regex' - Expecing 1 argument, found {}", arguments.len()));
		}

		let needle = operand_stack.dereference(&arguments[0]).as_string();
		let re = Regex::new(&needle).unwrap();
		self.compiled_regex_list.push(re);
		let rtn = self.compiled_regex_list.len() - 1;
		return CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Int, rtn as f64)));
	}

	pub fn builtin_locate(&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		if arguments.len() != 3 {
			abend!(format!("from Builtin Function 'Locate' - Expecing 3 arguments, found {}", arguments.len()));
		}

		let argslen = arguments.len();
		let haystack = operand_stack.dereference(&arguments[argslen-1]).as_string();
		let lookfor = operand_stack.dereference(&arguments[argslen-2]).as_string();
		let start = operand_stack.dereference(&arguments[argslen-3]).as_number() as usize;


		let start_limit = haystack.len() as i32 - lookfor.len() as i32;
		if start as i32 >= start_limit{
			return CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Int,-2.0)));
		}

		match haystack.get(start..).unwrap().find(&lookfor){
			None 		=> CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Int,-1.0))),
			Some(n)		=> CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Int,(n+start) as f64))),
		}

	}

	//	given a CplVar return its type as a string
	pub fn builtin_get_cpl_type(&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		if arguments.len() != 1 {
			abend!(format!("from Builtin Function 'Type' - Expecing 1 argument, found {}", arguments.len()));
		}

		let t = operand_stack.get_cpl_type(&arguments[0]);
		if t == "CplVarRef"{
			let var = operand_stack.dereference(&arguments[0]);
			let mut rtn = operand_stack.get_cpl_type(&var);
			rtn.push('&');
			CplVar::new(CplDataType::CplString(CplString::new(rtn)))			
		}else{
			CplVar::new(CplDataType::CplString(CplString::new(operand_stack.get_cpl_type(&arguments[0]).to_string())))
		}
	}


	//*******************************************************
	//***************  Collection Functions *****************
	//*******************************************************

	pub fn builtin_append (&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		if (arguments.len()) != 2{
			abend!(format!("from Builtin Function 'Append' - Expecting 2 arguments, found {}", arguments.len()));
		}
		
		let var = operand_stack.dereference(&arguments[0]);

		if let CplDataType::CplVarRef(var_ref) = &arguments[1].var{
			let operand_frames = operand_stack.operand_frames.last_mut().unwrap();
			if let CplDataType::CplArray(array) = &mut operand_frames.operand_blocks[var_ref.block_num].operand_block[var_ref.address].var{
				array.append(&var);
			}else if let CplDataType::CplDict(dict) = &mut operand_frames.operand_blocks[var_ref.block_num].operand_block[var_ref.address].var{
				dict.append(&var);
			}else{
				panic!("From:  builtin_append - Append:  Expecting parameter 1 to be a reference to an array or dictionary (e.g. append(&array,item_to_append))")
			}
			return CplVar::new(CplDataType::CplUninitialized(CplUninitialized::new()));
		}else{
			panic!("From:  builtin_append - Append:  Expecting parameter 1 to be a reference to an array or dictionary (e.g. Append(&array,item_to_append))")
		}
	}

	//	arguments[0] = dictionary
	//	returns an array of the keys contained in the dictionary
	pub fn builtin_keys (&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		if (arguments.len()) != 1{
			abend!(format!("from Builtin Function 'Keys' - Expecting 1 argument, found {}", arguments.len()));
		}
		if let CplDataType::CplVarRef(var_ref) = &arguments[0].var{
			if let CplDataType::CplDict(dict) = &mut operand_stack.operand_frames[var_ref.frame_num].operand_blocks[var_ref.block_num].operand_block[var_ref.address].var{
				return dict.keys();
			}else{
				panic!("From:  builtin_keys - Keys:  Expecting parameter 1 to be a reference to an array (e.g. Keys(&array))")
			}
		}else{
			panic!("From:  builtin_keys - Keys:  Expecting parameter 1 to be a reference to an array (e.g. Keys(&array))")
		}
	}

	pub fn builtin_sort (&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		if (arguments.len()) != 1{
			abend!(format!("from Builtin Function 'Sort' - Expecting 1 argument, found {}", arguments.len()));
		}

		if let CplDataType::CplVarRef(var_ref) = &arguments[0].var{
			if let CplDataType::CplArray(array) = &mut operand_stack.operand_frames[var_ref.frame_num].operand_blocks[var_ref.block_num].operand_block[var_ref.address].var{
				array.sort();
				return CplVar::new(CplDataType::CplUninitialized(CplUninitialized::new()));
			}else{
				panic!("From:  builtin_sort - Sort:  Expecting parameter 1 to be a reference to an array (e.g. Sort(&array))")
			}
		}else{
			panic!("From:  builtin_sort - Sort:  Expecting parameter 1 to be a reference to an array (e.g. Sort(&array))")
		}
	}


	pub fn builtin_push(&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		if (arguments.len()) != 2{
			abend!(format!("from Builtin Function 'Push' - Expecting 2 arguments, found {}", arguments.len()));
		}

		let new_value = operand_stack.dereference(&arguments[0]);

		if let CplDataType::CplVarRef(var_ref) = &arguments[1].var{
			let operand_frames = operand_stack.operand_frames.last_mut().unwrap();
			if let CplDataType::CplArray(array) = &mut operand_frames.operand_blocks[var_ref.block_num].operand_block[var_ref.address].var{
				array.push(&new_value);
				return CplVar::new(CplDataType::CplUninitialized(CplUninitialized::new()));
			}else{
				panic!("From builtin_push - Push:  Expecting parameter 1 to be a reference to an array (e.g. Push(&array,item_to_push))")	
			}
		}else{
			panic!("From builtin_push - Push:  Expecting parameter 1 to be a reference to an array (e.g. Push(&array,item_to_push))")
		}
	}

	pub fn builtin_pop(&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		if (arguments.len()) != 1{
			abend!(format!("from Builtin Function 'Pop' - Expecting 1 arguments, found {}", arguments.len()));
		}

		if let CplDataType::CplVarRef(var_ref) = &arguments[0].var{
			let operand_frames = operand_stack.operand_frames.last_mut().unwrap();
			if let CplDataType::CplArray(array) = &mut operand_frames.operand_blocks[var_ref.block_num].operand_block[var_ref.address].var{
				return array.pop();
			}else{
				abend!(format!("From builtin_pop Pop().  Expecting a reference to an array (e.g. Pop(&array))"));
			}
		}else{
			abend!(format!("From builtin_pop Pop().  Expecting a reference to an array (e.g. Pop(&array))"));
		}
	}


	pub fn builtin_insert(&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		if (arguments.len()) != 4{
			abend!(format!("from Builtin Function 'Insert' - Expecting 4 arguments, found {}", arguments.len()));
		}

		let key = operand_stack.dereference(&arguments[2]);
		let new_value = operand_stack.dereference(&arguments[1]);
		let update_flag = operand_stack.dereference(&arguments[0]);

		if let CplDataType::CplVarRef(var_ref) = &arguments[3].var{
			if let CplDataType::CplDict(dict) = &mut operand_stack.operand_frames[var_ref.frame_num].operand_blocks[var_ref.block_num].operand_block[var_ref.address].var{
				dict.insert_builtin(&key, &new_value, &update_flag);
			}else{
				panic!("From:  builtin_insert - Insert:  Expecting parameter 1 to be a reference to a dictionary (e.g. Insert(&dict,key,value)")
			}
			return CplVar::new(CplDataType::CplUninitialized(CplUninitialized::new()));
		}else{
			panic!("From:  builtin_insert - Insert:  Expecting parameter 1 to be a reference to a dictionary (e.g. Insert(&dict,key,value)")
		}
	}

	pub fn builtin_delete(&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		if (arguments.len()) != 2{
			abend!(format!("from Builtin Function 'Push' - Expecting 2 arguments, found {}", arguments.len()));
		}

		let key = operand_stack.dereference(&arguments[0]);

		if let CplDataType::CplVarRef(var_ref) = &arguments[1].var{
			// let operand_frames = operand_stack.operand_frames.last_mut().unwrap();
			if let CplDataType::CplDict(dict) = &mut operand_stack.operand_frames[var_ref.frame_num].operand_blocks[var_ref.block_num].operand_block[var_ref.address].var{
				dict.delete(&key);
				return CplVar::new(CplDataType::CplUninitialized(CplUninitialized::new()));
			}else{
				panic!("From builtin_push - Delete:  Expecting parameter 1 to be a reference to a dictionary(e.g. Delete(&dictionary,key))")	
			}
		}else{
			panic!("From builtin_push - Delete:  Expecting parameter 1 to be a reference to a dictionary(e.g. Delete(&dictionary,key))")	
		}
	}

	pub fn builtin_contains(&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		if (arguments.len()) != 2{
			abend!(format!("from Builtin Function 'Push' - Expecting 2 arguments, found {}", arguments.len()));
		}

		// let container = ;
		// let key = ;

		if let CplDataType::CplVarRef(ref var_ref) = &arguments[1].var{
			// let operand_frames = operand_stack.operand_frames.last_mut().unwrap();
			if let CplDataType::CplDict(dict) = &mut operand_stack.operand_frames[var_ref.frame_num].operand_blocks[var_ref.block_num].operand_block[var_ref.address].var{
				return dict.contains(&arguments[0]);
			}else{
				panic!("From builtin:Contains:  Parameter 1 is a reference but doesn't appear to be pointing a dictionary. It is {}",&mut operand_stack.operand_frames[var_ref.frame_num].operand_blocks[var_ref.block_num].operand_block[var_ref.address].var);
			}
		}else{
			panic!("From builtin:Contains:  Expecting parameter 1 to be a reference to a dictionary(e.g. Contains(&dictionary,key))")	
		}
	}



	//*******************************************************
	//***************  FILE I/O FUNCTIONS *******************
	//*******************************************************

	pub fn builtin_fopen (&mut self, arguments : &Vec<CplVar>, _operand_stack : &mut OperandStack) -> CplVar{
		if (arguments.len()) != 2{
			abend!(format!("from Builtin Function 'Fopen' - Expecing 2 arguments, found {}", arguments.len()));
		}
		let argslen = arguments.len();

		let file_name = arguments[argslen-1].as_string();
		let open_mode = arguments[argslen-2].as_string();
		
		match open_mode.as_bytes()[0] as char{
			'<' =>{
				self.opens.push(CplVar::new(CplDataType::CplFileReader(CplFileReader::new(&file_name, open_mode.as_str()))));
				return CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Int, (self.opens.len()-1) as f64)));
			},
			'>' =>{
				if open_mode.len() == 2 && open_mode.as_bytes()[1] as char == '>'{
					self.opens.push(CplVar::new(CplDataType::CplFileWriter(CplFileWriter::new(&file_name, true))));
					return CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Int, (self.opens.len()-1) as f64)));
				}else{
					self.opens.push(CplVar::new(CplDataType::CplFileWriter(CplFileWriter::new(&file_name, false))));
					return CplVar::new(CplDataType::CplNumber(CplNumber::new(RustDataType::Int, (self.opens.len()-1) as f64)));	
				}
			},
			_ => abend!(format!("Unknown open mode '{}'",open_mode)),
		}
	}

	pub fn builtin_fread (&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		if (arguments.len()) != 2{
			abend!(format!("from Builtin Function 'Fread' - Expecing 2 arguments, found {}", arguments.len()));
		}
		let argslen = arguments.len();

		if let CplDataType::CplNumber(file_num) = &arguments[argslen-1].var{
			if let CplDataType::CplVarRef(array_ref) = &arguments[argslen-2].var{
				if let CplDataType::CplFileReader(ref mut r) = self.opens[file_num.cpl_number as usize].var{
					if let CplDataType::CplArray(ref mut array) = 
							&mut operand_stack.operand_frames.get_mut(array_ref.frame_num).unwrap()
							.operand_blocks.get_mut(array_ref.block_num).unwrap()
							.operand_block.get_mut(array_ref.address).unwrap().var{
						r.read(array);
						CplVar::new(CplDataType::CplBool(CplBool::new(true)))	
					}else{						
						panic!("from 'Fread': Huston, we have a problem.  The array ref did not point at an Array");
					}
				}else{
					panic!("from 'Fread': Huston, we have a problem.  The file number did not point at a FileReader: {}", &arguments[argslen-1]);
				}
			}else{
				panic!("from 'Fread':  2nd parameter must be a reference to an array. Got: {}", &arguments[argslen-2]);
			}
		}else{
			abend!(format!("from 'Fread' - first argument must be a file number.  Got: {}", &arguments[argslen-1]));
		}
	}
	
	pub fn builtin_freadln (&mut self, arguments : &Vec<CplVar>, _operand_stack : &mut OperandStack) -> CplVar{
		if (arguments.len()) != 1{
			abend!(format!("from Builtin Function 'Freadln' - Expecing 1 arguments, found {}", arguments.len()));
		}
		let argslen = arguments.len();
		if let CplDataType::CplNumber(cpl_num) = &arguments[argslen-1].var{
			let file_num = cpl_num.cpl_number;
			if let CplDataType::CplFileReader(ref mut r) = self.opens[file_num as usize].var{
				if r.open_mode.len() > 1 {
					return CplVar::new(CplDataType::CplArray(r.read_csv()));
				}else{
					let mut line = String::new();
					r.readln(&mut line);
					return CplVar::new(CplDataType::CplString(CplString::new(line.trim().to_string())));	
				}
			};
		}
		CplVar::new(CplDataType::CplBool(CplBool::new(false)))
	}

	fn builtin_write_helper(&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack, writeln : bool) -> CplVar{
		if (arguments.len()) != 2{
			abend!(format!("from Builtin Function 'Fwrite' - Expecing 2 arguments, found {}", arguments.len()));
		}

		let argslen = arguments.len();

		if let CplDataType::CplNumber(cpl_num) = &arguments[argslen-1].var{
			let file_num = cpl_num.cpl_number;
			if let CplDataType::CplFileWriter(ref mut w) = self.opens[file_num as usize].var{
				match &arguments[argslen-2].var{
					CplDataType::CplString(s) => w.write(&s.cpl_string, writeln),
					CplDataType::CplNumber(n) => w.write(&n.cpl_number.to_string(), writeln),
					CplDataType::CplBool(b) => w.write(&b.cpl_bool.to_string(), writeln),
					CplDataType::CplVarRef(vr) => {
						let array = operand_stack.operand_frames.get(vr.frame_num).unwrap().operand_blocks.get(vr.block_num).unwrap().operand_block.get(vr.address).unwrap();
						w.write_array(array, writeln);					
					}
					_ => abend!(format!("Unable to write from a variable of type {}", &arguments[argslen-2].var)),
				}
			}
		}
		CplVar::new(CplDataType::CplBool(CplBool::new(true)))
	}

	pub fn builtin_fwrite (&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		self.builtin_write_helper(arguments, operand_stack, false)
	}

	pub fn builtin_fwriteln (&mut self, arguments : &Vec<CplVar>, operand_stack : &mut OperandStack) -> CplVar{
		self.builtin_write_helper(arguments, operand_stack, true)
	}	

	pub fn builtin_feof (&mut self, arguments : &Vec<CplVar>, _operand_stack : &mut OperandStack) -> CplVar{
		if (arguments.len()) != 1{
			abend!(format!("from Builtin Function 'Feof' - Expecing 1 arguments, found {}", arguments.len()));
		}
		let argslen = arguments.len();
		if let CplDataType::CplNumber(cpl_num) = &arguments[argslen-1].var{
			let file_num = cpl_num.cpl_number;
			if let CplDataType::CplFileReader(ref mut r) = self.opens[file_num as usize].var{
				return CplVar::new(CplDataType::CplBool(CplBool::new(r.eof_flag)));	
			};
		}
		CplVar::new(CplDataType::CplBool(CplBool::new(true)))
	}
}

