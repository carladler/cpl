
use std::env;
use cli::*;
use parser::*;
use cplvar::*;
use exec::*;
//use builtin::*;
use codeframe::*;



fn main() {
	let cl = env::args().collect();
	match CLI::new(&cl){
		None => return,
		Some(cli) => {
			if cli.is_debug_bit(SET_BACKTRACE){
				env::set_var("RUST_BACKTRACE", "1");
			}
			parse(&cli);
		}
	};
}

fn parse(cli : &CLI){
	let parse_result : (bool, Option<FrameMap>);
	{
		let mut _parser = Parser::new(cli);
		parse_result = _parser.parse_and_gen();
	}

	if !parse_result.0{
		return;
	}

	match parse_result.1{
		Some(mut f) => {
			let mut arguments : Vec<CplVar> = Vec::new();
			let mut operand_stack = OperandStack::new();
			let mut e = Executor::new(cli, &mut f, &mut arguments, &mut operand_stack, 0);
			let rtn = e.exec();
			match rtn.var{
				CplDataType::CplNumber(n) => if n.cpl_number as i32 != 0{
					println!("Program exiting with: {}", n.cpl_number);
					std::process::exit(n.cpl_number as i32); 
				},

				CplDataType::CplString(s) => {
					if s.cpl_string != "$$Synthetic$$"{
						println!("Program exit with: {}", s.cpl_string);
					}
				},

				CplDataType::CplUninitialized(_)=> {},
				
				_ => println!("Program exit with: {}", rtn),
			}
		},
		None => {},
	}
}