
use std::env;
use cli::*;
use parser::*;
use cplvar::*;
use exec::*;
//use builtin::*;
use codeframe::*;
//use opcode::*;
//use machineinstruction::*;
use std::time::SystemTime;
use std::time::Duration;

use std::fs::File;
use std::io::BufWriter;
use std::io::Write;



fn main() {
	let cl = env::args().collect();
	
	match CLI::new(&cl){
		None => return,
		Some(cli) => {
			if cli.is_debug_bit(SET_BACKTRACE){
				env::set_var("RUST_BACKTRACE", "1");
			}
			parse_and_run(&cli);
		}
	};
}

fn parse_and_run(cli : &CLI){
	let start_time = SystemTime::now();

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
			let mut operand_stack = OperandStack::new(cli.is_runtime_warnings());
			let mut runtime_data = RuntimeData::new();
			let mut e = Executor::new(cli, &mut f, &mut arguments, &mut operand_stack, 0, &mut runtime_data);
			let rtn = e.exec();

			//	if neither the -d27 switch or the -p switch is present then we're done
			if cli.is_runtime_stats_enabled(){
				eprintln!("\n========= Preparing Runtime Data for Display");
				let mut total_accumulated_time : u64 = 0;
				let accumulated_data : Vec<ReducedData> = e.reduce_event_data(&mut total_accumulated_time);
				display_stats(cli, &accumulated_data,start_time, total_accumulated_time);
				eprintln!("\n========= Cleaning up Runtime Data Repository");
			}

			e.exit(&rtn);
		},
		None => {},
	}
}

fn dump_stats_to_csv(csv_file: &str, reduced_data : &Vec<ReducedData>){

	let mut writer = BufWriter::new(File::create(csv_file).expect(&format!("--Unable to open {}--\n",csv_file)));
	// write header records
	writer.write_all(format!("OPCODE,").as_bytes()).expect("Unable to write data");
	writer.write_all(format!("MODE,").as_bytes()).expect("Unable to write data");
	writer.write_all(format!("QUAL,").as_bytes()).expect("Unable to write data");

	writer.write_all(format!("TOTAL ELAPSED,").as_bytes()).expect("Unable to write data");
	writer.write_all(format!("CALLS,").as_bytes()).expect("Unable to write data");
	writer.write_all(format!("\n").as_bytes()).expect("Unable to write data");

	for rd in reduced_data{
		writer.write_all(format!("{},",rd.opcode).as_bytes()).expect("Unable to write data");
		writer.write_all(format!("{},",rd.opcode_mode).as_bytes()).expect("Unable to write data");
		writer.write_all(format!("{},",rd.qual).as_bytes()).expect("Unable to write data");

		writer.write_all(format!("{},",rd.accum_duration.as_micros()).as_bytes()).expect("Unable to write data");
		writer.write_all(format!("{},",rd.accum_execution_count).as_bytes()).expect("Unable to write data");
		writer.write_all(format!("\n").as_bytes()).expect("Unable to write data");
	}
}


//	pass2 computes the percentage of the whole for each opcode and sorts it
//	by highest to lowest usage

fn pass2(reduced_data : &Vec<ReducedData>, total_accum_elapsed : u64, duration_list : &mut Vec<(ReducedData, f64)>){
	for rd in reduced_data{
		let percent = ((rd.accum_duration.as_micros() * 100) as f64) / total_accum_elapsed as f64;
		duration_list.push((rd.clone(), percent));
	}
}

fn format_duration(duration : Duration) -> String{
	const ONE_MINUTE : u128 = 1000000000 * 60;
	const ONE_SECOND : u128 = 1000000000;
	const ONE_MILISECOND : u128 = 1000000;
	const ONE_MICROSECOND : u128 = 1000;

	let nano = duration.as_nanos();
	if nano > ONE_MINUTE {
		return format!("{:3.1}mins", nano as f64 / ONE_MINUTE as f64);
	}else if nano > ONE_SECOND{
		return format!("{:3.2}s", nano as f64 / ONE_SECOND as f64);
	}else if nano > ONE_MILISECOND{
		return format!("{:3.1}ms", nano as f64 / ONE_MILISECOND as f64);
	}else{
		return format!("{:3.1}µs", nano as f64 / ONE_MICROSECOND as f64);
	}
}

fn display_stats(cli : &CLI, reduced_data: &Vec<ReducedData>, start_time : SystemTime, total_accumulated_time : u64){
	let total_run_time = start_time.elapsed().unwrap();

	let performance_csv_file = cli.get_performance_output_file();
	match performance_csv_file{
		None => {}
		Some(ref f) => dump_stats_to_csv(f, &reduced_data),
	}

	//	if user only wanted the csv file then we're done
	if !cli.is_debug_bit(DUMP_PERFORMANCE_STATS){return};

	let mut duration_list : Vec<(ReducedData, f64)> = Vec::new();
	pass2(reduced_data, total_accumulated_time, &mut duration_list);

	//	Now we need to create an inversion:
	let mut percent_list : Vec<(u64, ReducedData)> = Vec::new();

	for duration in &duration_list{
		let percent = (duration.1*100000.0) as u64;
		percent_list.push((percent, duration.0.clone()));
	}

	percent_list.sort();
	percent_list.reverse();
	
	eprintln!("\n*** Internal Runtime Statistics ***");
	//eprintln!("Total run time: {:?}", start_time.elapsed().unwrap());
	eprintln!("Total run time: {}", format_duration(total_run_time));
	//eprintln!("Total accumulated Elapsed time: {:?}",total_accumulated_time);
	eprintln!("Total accumulated Elapsed time: {}",format_duration(Duration::from_micros(total_accumulated_time)));
	eprintln!("{0: <20} | {1: <8} | {2: <8} | {3: <12} | {4: <9} | {5: <9} | {6: <9} ","opcode", "mode", "qual", "elapsed", "calls", "average", "percent");
	eprintln!("{0: <20} | {1: <8} | {2: <8} | {3: <12} | {4: <9} | {5: <9} | {6: <9} ","------", "----", "----", "-------", "-----", "-------", "-------");

	for percent in percent_list{
		let opcode_txt 						= format!("{}"		,percent.1.opcode);
		let opcode_mode_txt					= format!("{}"		,percent.1.opcode_mode);
		let qual_txt						= format!("{}"		,percent.1.qual);

		
		let accum_duration_txt : String;
		if percent.1.accum_duration.as_micros() < 1{
			accum_duration_txt 				= format!("<1µs");
		}else{
			accum_duration_txt = format_duration(percent.1.accum_duration);
			// accum_duration_txt 				= format!("{:?}"	,percent.1.accum_duration);
		}
		
		
		let accum_execution_count_txt 		= format!("{}"		,percent.1.accum_execution_count);
		let accum_average_duration_txt 		= format!("{}"		,format_duration(percent.1.accum_average_duration));

		let actual_percent = percent.0 as f64 / 100000.0;

		let percent_txt 					= format!("{:2.1}%"	,actual_percent);

		eprintln!("{0: <20} | {1: <8} | {2: <8} | {3: <12} | {4: <9} | {5: <9} | {6: <9}"
				,opcode_txt
				,opcode_mode_txt
				,qual_txt
				,accum_duration_txt
				,accum_execution_count_txt
				,accum_average_duration_txt
				,percent_txt
		);
	}
}