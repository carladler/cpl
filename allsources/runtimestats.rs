use opcode::*;
use std::time::Duration;
use std::time::Instant;
use machineinstruction::*;
use std::collections::HashMap;

#[derive(Clone)]
pub struct EventPayload{
	pub event_count : u64,
	pub event_accum_duration : Duration,
}

impl EventPayload{
	pub fn new(event_count : u64, event_accum_duration : Duration) -> EventPayload{
		EventPayload{
			event_count : event_count,
			event_accum_duration : event_accum_duration,
		}
	}
}

#[derive(Clone)]
pub struct RuntimeData{
	pub mark : Instant,
	//	number of calls, accumulated duration
	pub accumulated_events : HashMap<InstructionKey, EventPayload>,
}

impl RuntimeData{
	pub fn new() -> RuntimeData{
		RuntimeData{
			mark : Instant::now(),
			accumulated_events : HashMap::new(),
		}
	}

	pub fn mark_begin (&mut self){
		self.mark = Instant::now();
	}

	pub fn mark_end (&mut self, machine_instruction : &MachineInstruction, runtime_data_qual : String){
		let elapsed = self.mark.elapsed();
		let key = InstructionKey::to_key(machine_instruction, runtime_data_qual.clone());

		match self.accumulated_events.get(&key){
			None => {
				self.accumulated_events.insert(key, EventPayload::new(1, elapsed));
			}
			Some(event_payload) => {
				self.accumulated_events.insert(key, EventPayload::new(event_payload.event_count + 1, event_payload.event_accum_duration + elapsed));
			}
		}
	}
}


#[derive(PartialOrd, Ord, PartialEq, Clone, Eq)]
pub struct EventSummary{
	pub opcode : Opcode,
	pub opcode_mode : OpcodeMode,
	pub qual : String,
	pub accum_duration : Duration,
	pub accum_execution_count : u64,
	pub accum_average_duration : Duration,
}

impl EventSummary{
	pub fn new(instruction_key : &InstructionKey, accum_duration : Duration, accum_execution_count : u64, accum_average_duration : Duration) -> EventSummary{
		EventSummary{
			opcode : instruction_key.opcode,
			opcode_mode : instruction_key.opcode_mode,
			qual : instruction_key.qual.clone(),
			accum_duration : accum_duration,
			accum_execution_count : accum_execution_count,
			accum_average_duration : accum_average_duration,
		}
	}
}

#[derive(Hash, Eq, PartialEq, Clone, Ord, PartialOrd)]
pub struct InstructionKey{
	pub opcode : Opcode,
	pub opcode_mode : OpcodeMode,
	pub qual : String,
}

impl InstructionKey{
	pub fn to_key (instruction : &MachineInstruction, qual : String) -> InstructionKey{
		InstructionKey{
			opcode : instruction.opcode,
			opcode_mode : instruction.opcode_mode,
			qual : qual,
		}
	}

	pub fn to_key2 (opcode : Opcode, opcode_mode : OpcodeMode, qual : String) -> InstructionKey{
		InstructionKey{
			opcode : opcode,
			opcode_mode : opcode_mode,
			qual : qual,
		}
	}
}
