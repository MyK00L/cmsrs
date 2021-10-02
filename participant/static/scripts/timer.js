"use strict";
function millis_to_string(millis) {
	let days = Math.floor(millis / (1000 * 60 * 60 * 24));
	let hours = Math.floor((millis % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
	let minutes = Math.floor((millis % (1000 * 60 * 60)) / (1000 * 60));
	let seconds = Math.floor((millis % (1000 * 60)) / 1000);
	if(days>0) {
		return days+"d "+hours+"h "+minutes+"m ";
	} else {
		return Math.floor(hours/10)+""+hours%10+":"+Math.floor(minutes/10)+""+minutes%10+":"+Math.floor(seconds/10)+""+seconds%10;
	}
}
let now = Date.now();
const stage = now<start_time ? 0 : now<end_time ? 1 : 2;
let x = setInterval(function() {
	now = Date.now();
	if(now<start_time) {
		if(stage!=0) {
			window.location.reload();
		}
		document.getElementById("timer").innerHTML="start in "+millis_to_string(start_time-now);
	} else if(now<end_time) {
		if(stage!=1) {
			window.location.reload();
		}
		document.getElementById("timer").innerHTML="end in "+millis_to_string(end_time-now);
	} else {
		if(stage!=2) {
			window.location.reload();
		}
		document.getElementById("timer").innerHTML="ended "+millis_to_string(now-end_time);
	}
}, 1000);
