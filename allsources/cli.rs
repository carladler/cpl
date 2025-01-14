use std::collections::HashMap;


pub const TRACE_TOKENIZER : u32 = 0;
pub const TRACE_PARSER_NEXT_TOKEN : u32 = 1;
pub const TRACE_PARSER_STATES : u32 = 2;
pub const TRACE_PARSE_LOOP : u32 = 3;
pub const TRACE_MODEL_ADD : u32 = 4;
pub const TRACE_INFIX_TO_POSTFIX : u32 = 5;
pub const TRACE_INFIX_TO_POSTFIX_DRIVER : u32 = 6;
pub const TRACE_CODE_GEN : u32 = 7;
pub const TRACE_CODE_GEN_ADD_INSTRUCTION : u32 = 8;
pub const TRACE_EXEC : u32 = 9;
pub const TRACE_EXEC_DISPATCHER : u32 = 10;
pub const TRACE_STATEMENT_ADD : u32 = 11;
pub const DISPLAY_RUNTIME : u32 = 12;

pub const DUMP_POSTFIX_EXPRESSION : u32 = 20;
pub const DUMP_OPERATOR_STACK : u32 = 21;
pub const DUMP_GEN_CODE : u32 = 22;
pub const DUMP_SYMBOL_TABLE : u32 = 23;
pub const DUMP_OPERANDS : u32 = 24;
pub const DUMP_OPERANDS_DISPATCH : u32 = 25;
pub const DUMP_STRUCTS : u32 = 26;
pub const DUMP_PERFORMANCE_STATS : u32 = 27;

pub const SET_BACKTRACE : u32 = 30;
pub const INSERT_DIAG_COMMENTS : u32 = 31;

fn usage_d_swiches(){
	eprintln!("\n         -d values:");
	eprintln!("            TRACE_TOKENIZER = {}",TRACE_TOKENIZER);
	eprintln!("            TRACE_PARSER_NEXT_TOKEN = {}",TRACE_PARSER_NEXT_TOKEN);
	eprintln!("            TRACE_PARSER_STATES = {}",TRACE_PARSER_STATES);
	eprintln!("            TRACE_PARSE_LOOP = {}",TRACE_PARSE_LOOP);
	eprintln!("            TRACE_MODEL_ADD = {}",TRACE_MODEL_ADD);
	eprintln!("            TRACE_INFIX_TO_POSTFIX = {}",TRACE_INFIX_TO_POSTFIX);
	eprintln!("            TRACE_INFIX_TO_POSTFIX_DRIVER= {}", TRACE_INFIX_TO_POSTFIX_DRIVER);
	eprintln!("            TRACE_CODE_GEN = {}",TRACE_CODE_GEN);
	eprintln!("            TRACE_CODE_GEN_ADD_INSTRUCTION = {}",TRACE_CODE_GEN_ADD_INSTRUCTION);
	eprintln!("            TRACE_EXEC = {}", TRACE_EXEC);
	eprintln!("            TRACE_EXEC_DISPATCHER= {}", TRACE_EXEC_DISPATCHER);
	eprintln!("            TRACE_STATEMENT_ADD = {}", TRACE_STATEMENT_ADD);
	eprintln!("");
	eprintln!("            DISPLAY_RUNTIME = {}", DISPLAY_RUNTIME);
	eprintln!("");
	eprintln!("            DUMP_POSTFIX_EXPRESSION = {}",DUMP_POSTFIX_EXPRESSION);
	eprintln!("            DUMP_OPERATOR_STACK = {}", DUMP_OPERATOR_STACK);
	eprintln!("            DUMP_GEN_CODE = {}",DUMP_GEN_CODE);
	eprintln!("            DUMP_SYMBOL_TABLE = {}",DUMP_SYMBOL_TABLE);
	eprintln!("            DUMP_OPERANDS = {}",DUMP_OPERANDS);
	eprintln!("            DUMP_OPERANDS_DISPATCH = {}",DUMP_OPERANDS_DISPATCH);
	eprintln!("            DUMP_STRUCTS = {}",DUMP_STRUCTS);
	eprintln!("");
	eprintln!("            DUMP_PERFORMANCE_STATS = {}",DUMP_PERFORMANCE_STATS);
	eprintln!("");
	eprintln!("            SET_BACKTRACE = {}",SET_BACKTRACE);
	eprintln!("            INSERT_DIAG_COMMENTS = {}",INSERT_DIAG_COMMENTS);
}

fn usage_all(){
	eprintln!("\nWELCOME TO CARL'S PROGRAMMING LANGAUGE (CPL)\n");
	eprintln!("\ncpl <source> <switches> [<arguments>] \n");
	eprintln!("    <source>      ::= file containing source code");
	eprintln!("    <switches>    ::= [-<sw>[<sw_parameter>]]\n");
	eprintln!("         <sw>     := 'd<debug bit>[+<debug bit>]' (debug)");
	eprintln!("                   | 'w' (Warn runtime errors)");
	eprintln!("                   | 'h' (help/usage))");
	eprintln!("                   | 'o'<file> (output file)");
	eprintln!("                   | 'p'<file> (performance stats csv file)");
	eprintln!("   <arguments>    ::= a space separated list of arguments passed to ENTRY function");
	
	usage_d_swiches();

	eprintln!("\n");
	eprintln!("        -d values can be 'added' together to specify multiple switches; e.g. -d1+4 will set bits 1 and 4");
	eprintln!("        -w switch tells the executor to println a warning message instead of abend if possible");
	eprintln!("\n");
	eprintln!("        NOTE: the source file may appear anywhere in the argument list but must be the first non-switch");
	eprintln!("        <switches can appear anywhere on the command line");

	eprintln!("\n    NOTE: no space between switch name and its parameter (e.g. use -ojunk instead of -o junk)");
}

pub fn is_debug_bit(debug_bits : u32, bit_no : u32) -> bool{
	(debug_bits & (1 << bit_no)) > 0
}



pub struct CLI<'a>{
	switches : HashMap<char,String>,
	debug_bits : Vec<u32>,
	cl_index : usize,
	cl : & 'a Vec<String>,
	pub cl_args : Vec<String>,
	runtime_warnings : i32,
}

impl<'a> CLI<'a>{
	pub fn new(cl : &Vec<String>) -> Option<CLI>{

		let mut cli = CLI{
			switches : HashMap::new(),
			debug_bits : Vec::new(),
			cl_index : 1,
			cl : cl,
			cl_args : Vec::new(),
			runtime_warnings : 0,
		};

		if cli.cl.len() < 2 {
			eprintln!("Source file name expected as first argument");
			usage_all();
			return None;
		}

		//  now look for switches
		while cli.cl_index < cli.cl.len(){
			//eprintln!("ix: {} arg: {}", cli.cl_index, cli.cl[cli.cl_index]);
			if cli.cl[cli.cl_index].as_bytes()[0] as char == '-'{
				if cli.cl[cli.cl_index].len() == 1{
					usage_d_swiches();return None;
				}
				match cl[cli.cl_index].as_bytes()[1] as char{
					'd' | 'D' => {let ls = cli.get_d_switch_parameter(); cli.switch_insert('d',&ls)},
					'h' | 'H' => {usage_all();return None;},
					'o' | 'O' => {let ls = cli.get_switch_parameter(false); cli.switch_insert('o',&ls)},
					'w' | 'W' => cli.runtime_warnings += 1,
					'p' | 'P' => {let ls = cli.get_switch_parameter(false); cli.switch_insert('p',&ls)}
					'-' => {},
					_   =>	{
								eprintln!("Switch {} ignored",cl[cli.cl_index].as_bytes()[1] as char);
							},
				};
			}else{
				if !cli.switches.contains_key(&'s'){
					let ls = cli.get_switch_parameter(true);
					cli.switch_insert('s', &ls);
				}else{
					//	we got to here because a command line argument was not
					//  a switch AND we've already seen the name of the CPL source
					//  so we're assuming this is the name of a data file which
					//  the CPL program will want to process.  With any luck, these name
					//  will appear in an array passed to the ENTRY function.
					//  We don't know if these are really files or just random bits of
					//  of text.  And, really we don't have to care here.  We'll let
					//	CPL verbs and built-ins deal with this.
					cli.cl_args.push(cl[cli.cl_index].to_string());
				}
			}
			cli.cl_index+=1;
		}

		Some(cli)
	}

	fn switch_insert(&mut self, c : char, v : &str){
		self.switches.insert(c,v.to_string());
	}

	fn get_switch_parameter(&mut self, switchless : bool) -> String{
		let mut s = String::new();
		let mut i=2;
		if switchless{
			i = 0;
		}
		while  i < self.cl[self.cl_index].len(){
			s.push(self.cl[self.cl_index].as_bytes()[i] as char);
			i+=1;
		}	
		s
	}

	//  parses a simple bit expression.  E.G  1+4 sets bits 0 and 4 which returns the number "5"
	fn get_d_switch_parameter(&mut self) -> String{
		if self.cl.len() < 3{
			return String::new();
		}
		const RADIX: u32 = 10;
		let mut rslt : u32 = 0;
		let mut v : u32 = 0;
		let mut count = 0;
		let arg = self.cl[self.cl_index].clone();
		let mut i = 2;
		while i < arg.len(){
			match arg.as_bytes()[i] as char{
				'0'..='9' => {count += 1; v = v * 10 + (arg.as_bytes()[i] as char).to_digit(RADIX).unwrap();},
				'+' => {self.debug_bits.push(v); rslt += 1 << v; v = 0;count = 0; },
				_   => {},
			}
			i += 1;
		}
		if count != 0{
			self.debug_bits.push(v);
			rslt += 1 << v;
		}
		let s = rslt.to_string();
		s
	}

	pub fn source(&self) -> &String{
		&self.switches.get(&'s').unwrap()
	}

	pub fn debug(&self) -> u32{
		if self.switches.contains_key(&'d'){
			let p = self.switches.get(&'d').unwrap();
			if !p.trim().is_empty(){
				return p.trim().parse().unwrap();
			}
		}
		0u32
	}

	pub fn debug_bits(&self) -> String{
		let mut rtn = String::new();
		if self.debug_bits.len() == 0{
			return "None".to_string();
		}

		rtn.push_str(&self.debug_bits[0].to_string());

		if self.debug_bits.len() <= 1{
			return rtn;
		}

		for bit in &self.debug_bits[1..]{
			rtn.push('+');
			rtn.push_str(&bit.to_string());
		}

		return rtn;
	}

	pub fn is_debug_bit(&self, bit_no : u32) -> bool{
		(self.debug() & (1 << bit_no)) > 0
	}

	pub fn switch(&self, sw : char) ->&str{
		match self.switches.get(&sw){
			Some(v) => return v,
			None	=> return "NONE",
		}
	}

	pub fn is_runtime_warnings(&self) -> bool{
		if self.runtime_warnings > 0{
			return true;
		}
		return false;
	}

	pub fn is_runtime_stats_enabled(&self) -> bool{
		self.is_debug_bit(DUMP_PERFORMANCE_STATS) || self.get_performance_output_file() != None
	}

	pub fn get_performance_output_file (&self) -> Option<&str>{
		let c : char = 'p';
		if self.switches.contains_key (&c){
			return Some(self.switches.get(&c).unwrap());
		}else{
			return None;
		}
	}
}
