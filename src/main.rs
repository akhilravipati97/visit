/**
 * Author: Akhil Ravipati
 * 
 * I am new to rust. You'll have to forgive me for any un-idiomatic atrocities I may have committed here.
 * But if the program runs, and the compiler is happy, I guess I did do some things right?
 * 
 * Took help from:
 * 1. For the process: https://stackoverflow.com/questions/22077802/simple-c-example-of-doing-an-http-post-and-consuming-the-response
 * 2. For handling the stream: https://riptutorial.com/rust/example/4404/a-simple-tcp-client-and-server-application--echo
 * 
 */

use clap::{App, Arg, ArgMatches};
use std::net::{TcpStream, Shutdown};
use std::process::{exit, Command};
use std::str;
use regex::Regex;
use std::io::{Read, Write};
use url::{Url, Host};
use std::time::Duration;
use std::io::ErrorKind::{TimedOut, WouldBlock};
use std::time::Instant;

struct ProfileInfo {
    size_in_bytes: usize,
    request_time_ms: u128,
    success: bool,
    resp_code: u16
}

const TIMEOUT_SEC: u8 = 2;

fn main() {
    // Parse user args and get the url and profile count
    let (url, profile_count) = get_url_and_profile_count();
    let address = get_ip_address(url.host().unwrap());
    let get_request = format!("GET {} HTTP/1.1\r\nHost: {}\r\n\r\n", url.path(), url.host().unwrap());
    let resp_code_regex = Regex::new("^[^\\s]+\\s+([^\\s]+)\\s+[^\\s]+\\s*.*$").unwrap();
    // println!("The GET request: \n{}", get_request);

    if profile_count > 0 {
        print!("\nProfiling\n\n");
        let mut profile_infos: Vec<ProfileInfo> = (0..profile_count)
            .enumerate()
            .inspect(|(i1,_)| println!("Request {}", i1+1))
            .map( |_| perform_request(&address, &get_request, &resp_code_regex))
            .collect();

        print_profile_info_stats(&mut profile_infos);
    }
    else {
        perform_request(&address, &get_request, &resp_code_regex);
    }
    exit(0);
}

/**
 * Calculare and print profiling stats
 */
fn print_profile_info_stats(profile_infos: &mut Vec<ProfileInfo>) {
    println!("\n\n========Profiling Information ==============================");
    profile_infos.sort_unstable_by_key(|pi| pi.request_time_ms);
    let num_requests = profile_infos.len();
    println!("Number of requests: {}", num_requests);

    let fastest_time_ms = profile_infos.first().unwrap().request_time_ms;
    println!("Fastest time (ms): {}", fastest_time_ms);

    let slowest_time_ms = profile_infos.last().unwrap().request_time_ms;
    println!("Slowest time (ms): {}", slowest_time_ms);

    let mean_time_ms = profile_infos.iter()
        .map(|pi| pi.request_time_ms)
        .fold(0, | a, b| a + b) as f64 
        / num_requests as f64;
    println!("Mean response time (ms): {}", mean_time_ms);

    let mid_idx = num_requests/2;
    let median_time_ms = match num_requests % 2 {
        0 => {
            (profile_infos.get(mid_idx).unwrap().request_time_ms + profile_infos.get(mid_idx-1).unwrap().request_time_ms) as f64 / 2.0
        },
        1 => {
            profile_infos.get(mid_idx).unwrap().request_time_ms as f64
        },
        _ => 0.0 as f64
    };
    println!("Median response time (ms): {}", median_time_ms);

    let success_rate = (profile_infos.iter().filter(|pi| pi.success).count() * 100) as f64 / num_requests as f64;
    println!("Success rate: {}", success_rate);

    let error_codes: Vec<u16> = profile_infos.iter()
        .filter(|pi| pi.resp_code != 200)
        .map(|pi| pi.resp_code)
        .collect();
    println!("Non success error codes: {:?}", error_codes);

    let smallest_response = profile_infos.iter().min_by_key(|pi| pi.size_in_bytes).unwrap().size_in_bytes;
    println!("Smallest response size (bytes): {}", smallest_response);

    let largest_response = profile_infos.iter().max_by_key(|pi| pi.size_in_bytes).unwrap().size_in_bytes;
    println!("Largest response size (bytes): {}", largest_response);
    println!("============================================================");

}

fn get_url_and_profile_count() -> (Url, i32) {
    let args = App::new("visit")
        .version("1.0")
        .author("Akhil Ravipati")
        .about("Perform and profile GET requests")
        .arg(Arg::with_name("url")
            .long("url")
            .takes_value(true)
            .required(true)
            .help("URL to visit with a GET request"))
        .arg(Arg::with_name("profile")
            .long("profile")
            .required(false)
            .takes_value(true)
            .help("Profile the request for the provided number of times and obtain stats"))
        .get_matches();

    return (get_url(&args), get_profile_count(&args));
}

fn get_url(args: &ArgMatches) -> Url {
    match Url::parse(args.value_of("url").unwrap()) {
        Ok(parsed_url) => {
            println!("Passed URL: {}", parsed_url);
            parsed_url
        },
        Err(err) => {
            println!("URL Parse Error: {}", err);
            exit(-1);
        }
    }
}

/**
 * Return the profile count passed by the user.
 * Default value is -1.
 */
fn get_profile_count(args: &ArgMatches) -> i32 {
    match args.is_present("profile") {
        true => match args.value_of("profile").unwrap().to_string().parse::<i32>() {
            Ok(parsed_num) => { 
                println!("Profiling turned on, with count: {}", parsed_num); 
                parsed_num
            },
            Err(err) => {
                println!("Failed to parse the provided profile count {:?}", err); 
                exit(-1);
            }
        },
        false => -1
    }
}

/**
 * Fetch IP address via a DNS lookup for the provided host.
 * IPv4 is returned. In case IPv4 is not present, it will error out.
 * 
 * NOTE: Uses OS based shell commands to perform an nslookup, as 
 * the correct API to perform this via standard library could not be found at the time
 * of writing.
 */
fn get_ip_address(hostname: Host<&str>) -> String {
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
                .args(&["/C", &format!("nslookup {}", hostname)])
                .output()
                .expect("failed to execute process")
    } else { // didn't get around to check this on linux yet
        Command::new("sh")
                .arg("-c")
                .arg(&format!("nslookup {}", hostname))
                .output()
                .expect("failed to execute process")
    };

    let outputstr = str::from_utf8(&output.stdout).unwrap();
    let ip_regex = Regex::new("([0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3})").unwrap(); // ipv6 ignored
    let address = &ip_regex.captures_iter(outputstr).skip(1).next().unwrap()[0];
    println!("Found address: {:?}", &address);
    return address.to_string();
}

/**
 * Performs the request by writing it to a TCPStream and reading the response.
 * Default read/write timeout is 2sec.
 * Returns the Profile Information for the request which include details about the
 * request body size, time taken for the request, repsonse code etc.
 */
fn perform_request(address: &String, get_request: &String, resp_code_regex: &Regex) -> ProfileInfo {
    let start = Instant::now();
    let mut size_in_bytes: usize = 0;
    let mut success = true;
    let mut resp_code = 200;
    let mut request_time_ms: u128 = 0;

    // Connect to the resolved address provided on the default http port
    match &mut TcpStream::connect(format!("{}:80", address)) {
        Ok(mstream) => {
            // To prevent indefinite blocking of the read call on the stream, set timeouts
            let _res = mstream.set_read_timeout(Some(Duration::new(TIMEOUT_SEC as u64, 0)));
            let _res = mstream.set_write_timeout(Some(Duration::new(TIMEOUT_SEC as u64, 0)));

            // Write the provided GET request
            match mstream.write(get_request.as_bytes()) {
                Ok(_) => {},
                Err(__) => {println!("Error while writing: {}\nExiting..", __); exit(-1);}
            }

            // Read the response into a buffer
            let mut buffer = String::new();
            match (mstream).read_to_string(&mut buffer) {
                Ok(_) => { }
                Err(err) => {
                    match err.kind() {
                        TimedOut | WouldBlock => {}, // Read timeout/blocking setting related errors - so no action required
                        other_error => {println!("Error occurred: {:?}", other_error); exit(-1);}
                    }
                }
            }

            // Parse the response in the buffer
            request_time_ms += start.elapsed().as_millis();
            if buffer.len() > 0 {
                // Since the timeout causes some delay, subtract that from the elapsed time
                request_time_ms -= 1000 * (TIMEOUT_SEC  as u128);
                // Parse the response code
                let (s, r) = get_success_and_resp_code(&buffer[0..buffer.find("\r\n").unwrap()].to_string(), &resp_code_regex);
                success = s; resp_code = r;
                // Find out the start of the response body
                let idx = buffer.find("\r\n\r\n").unwrap_or(buffer.len()) + 4;
                if idx < buffer.len() {
                    size_in_bytes = buffer.len() - idx;
                    println!("Response body: \n{}\n", &buffer[idx..]);
                    
                }
            }
            
            // Close the TCPStream/Socket
            match mstream.shutdown(Shutdown::Both) {
                Ok(_) => {},
                Err(__) => {
                    println!("Error while closing stream: {:?}", __); 
                } 
            }
        }
        Err(err) => {
            println!("Couldn't connect to server: {:?}", err);
            exit(-1);
        }
    }
    return ProfileInfo { size_in_bytes, request_time_ms, success, resp_code };
}

fn get_success_and_resp_code(response: &String, resp_code_regex: &Regex) -> (bool, u16) {
    let captures = resp_code_regex.captures_iter(response).next().unwrap();
    let resp_code: u16 = captures[1].parse::<u16>().unwrap();
    let success = match resp_code { 200 => true, _ =>  false};
    return (success, resp_code);
}