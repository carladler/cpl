//	A bunch of abnormal exit functions

pub fn abend(msg : &str){
	println!("{}",msg);
	std::process::exit(1);
}
