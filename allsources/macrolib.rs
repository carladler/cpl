// ********  CPL Macro Library  ********
//
#[macro_export]
macro_rules! abend{
	($x:expr) => {
		{
			eprintln!("Cpl Abend {}",$x);
			std::process::exit(1);
		}
	};
 }

//  #[macro_export]
// macro_rules! mtest {
//     ( $( $x:expr ),* ) => {
//         {
// 			println!(
//                 $(
//                     "{}",($x)
//         		)*
// 			)
//         }
//     };
// }