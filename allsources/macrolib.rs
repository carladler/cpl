// ********  CPL Macro Library  ********
//
#![allow(unused_macros)]
#[macro_export]
macro_rules! abend{
	($x:expr) => {
		{
			eprintln!("Cpl Abend {}",$x);
			std::process::exit(1);
		}
	};
 }

//  macro_rules! dbg{
// 	($x:expr) => {
// 		{
			
// 		}
// 	};	
//  }
