"use strict";

function millis_to_string(millis) {
	const days = Math.floor(millis / (1000 * 60 * 60 * 24));
	const hours = Math.floor((millis % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
	const minutes = Math.floor((millis % (1000 * 60 * 60)) / (1000 * 60));
	const seconds = Math.floor((millis % (1000 * 60)) / 1000);
	if(days>0) {
		return days+"d "+hours+"h "+minutes+"m ";
	} else {
		return Math.floor(hours/10)+""+hours%10+":"+Math.floor(minutes/10)+""+minutes%10+":"+Math.floor(seconds/10)+""+seconds%10;
	}
}
let now = Date.now();

const stage = now<start_time ? 0 : now<end_time ? 1 : 2;
const stage_string = ["start in ","end in ","ended "];
function update_timer() {
	now = Date.now();
	const new_stage = now<start_time ? 0 : now<end_time ? 1 : 2;
	if(stage!=new_stage) {
			window.location.reload();
	} else {
		document.getElementById("timer").innerHTML=stage_string[stage]+millis_to_string(stage==0 ? start_time-now : stage==1 ? end_time-now : now-end_time);
	}
}

function start_timer() {
	update_timer();
	let x = setInterval(update_timer, 1000);
}

