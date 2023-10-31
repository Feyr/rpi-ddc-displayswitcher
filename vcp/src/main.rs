#![allow(dead_code)]
#![allow(unused_variables)]

use rppal::gpio::Gpio;
use rppal::gpio::Trigger::FallingEdge;

use rppal::i2c::I2c;
//use rppal::pwm::{Channel, Pwm};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
//use rppal::uart::{Parity, Uart};
use std::thread;
use std::time::{SystemTime, Duration};
use std::error::Error;
use std::process::Command;
use std::str;

use std::str::FromStr;
use std::collections::VecDeque;
use std::cmp::{min, max};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

extern crate daemonize;
use std::fs::File;
use daemonize::Daemonize;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    daemonize: bool,
}


pub struct Hw {
    pub gpio: Gpio,
    pub i2c: I2c,
    //pub pwm: Pwm,
    pub spi: Spi,
    //pub uart: Uart
    pub brightness: Brightness,

}

impl Hw {
    pub fn new() -> Hw {
        Hw {
            gpio: Gpio::new().unwrap(),
            i2c: I2c::new().unwrap(),
            // pwm: Pwm::new(Channel::Pwm0).unwrap(),
            spi: Spi::new(Bus::Spi0, SlaveSelect::Ss0, 16_000_000, Mode::Mode0).unwrap(),
            //uart: Uart::new(115_200, Parity::None, 8, 1).unwrap(),
            brightness: Brightness::new(),
            }
        }
    }

    
const DP1: &str = "0x0f";
const DP2: &str = "0x10";
// const JOY_UP: u8= 6;
// const JOY_DOWN: u8= 19;
const JOY_LEFT: u8= 5;
const JOY_RIGHT: u8= 26;
// const JOY_PRESS: u8= 13;
const KEY1: u8= 21;
// const KEY2: u8= 20;
const KEY3: u8= 16;
// const MOSI: u8 = 10;
// const SCLK: u8 = 11;
// const CS: u8 = 8;
// const DC: u8 = 25;
// const RST: u8 = 27;
const BL: u8 = 24;


pub struct Brightness {
    pub brightness: i16,
}

impl Brightness {
    pub fn new() -> Brightness {
        let mut b = Brightness {
            brightness: 50
        };
        let _ = b.get_brightness();
        b
    }

    pub fn get_brightness(&mut self) -> Result<i16, Box<dyn Error>> {
            let mut cmd = Command::new("ddcutil");
            let cmd = cmd.args(["getvcp", "10"]);
        
            let output = cmd.output()
                                    .expect("failed to execute process");
            let s = match str::from_utf8(&output.stdout) {
                Ok(v) => v,
                Err(e) => panic!("Invalid utf-8: {}", e)
            };

            let mut s = s.split('=').collect::<VecDeque<_>>();
            _= s.pop_front();
            let s = s.pop_front().unwrap().trim();
            let s = s.split(',').collect::<VecDeque<_>>().pop_front().unwrap();
            println!("found: {}", s);
            let val = i16::from_str(s).unwrap();
                      
            self.brightness = val;
            Ok(val)    
    }

    pub fn compute_brightness(&self, sign: char, val: i16) -> i16 {
        match sign {
            '-' => max(self.brightness - val, 0),
            '+' => min(self.brightness + val, 100),
            _ => min(max(val, 0), 100),
        }
    }

    pub fn set_brightness(&mut self, sign: char, val: i16) {
        let new_val = self.compute_brightness(sign, val);
        let mut cmd = Command::new("ddcutil");
        let cmd = cmd.args(["setvcp", "10", &new_val.to_string()]);
    
        
        let output = cmd.output()
                                .expect("failed to execute process");
        println!("Setting brightness {}:  {:?}: {}: {}", self.brightness, cmd.get_args(), output.status, String::from_utf8(output.stdout).unwrap() );
        self.brightness = new_val;

    }


}

#[cfg(test)]
mod brightness_tests {
    use crate::Brightness;
    #[test]
    fn test() {
        let mut brightness= Brightness::new();
        brightness.brightness = 50;
        assert_eq!(brightness.brightness, 50);
        assert_eq!(brightness.compute_brightness('+', 50), 100);
        brightness.brightness = 100;
        assert_eq!(brightness.compute_brightness('+', 50), 100);
        brightness.brightness = 100;
        assert_eq!(brightness.compute_brightness('-', 50), 50);
        brightness.brightness = 50;
        assert_eq!(brightness.compute_brightness('-', 50), 0);
        brightness.brightness = 0;
        assert_eq!(brightness.compute_brightness('-', 50), 0);
        

    }


}

fn setdp(dp: &str)  {
    let mut cmd = Command::new("ddcutil");
    let cmd = cmd.args(["setvcp", "60", &dp]);

    let output = cmd.output()
                            .expect("failed to execute process");

    println!("Setting DP {}:  {:?}: {}: {}", dp, cmd.get_args(), output.status, String::from_utf8(output.stdout).unwrap() );

    
}

fn getdp() -> Result<String, Box<dyn Error>> {
    let mut cmd = Command::new("ddcutil");
    let cmd = cmd.args(["getvcp", "60"]);

    let output = cmd.output()
                            .expect("failed to execute process");
    let s = match str::from_utf8(&output.stdout) {
        Ok(v) => v,
        Err(e) => panic!("Invalid utf-8: {}", e)
    };
    Ok(s.to_string())
}

fn daemonize() {
    let stdout = File::create("log").unwrap();
    let stderr = File::create("log.err").unwrap();

    let daemonize = Daemonize::new()
        .umask(0o077)    // Set umask, `0o027` by default.
        .stdout(stdout)  // Redirect stdout to `/tmp/daemon.out`.
        .stderr(stderr)  // Redirect stderr to `/tmp/daemon.err`.
        .privileged_action(|| "Executed before drop privileges");

    match daemonize.start() {
        Ok(_) => println!("Success, daemonized"),
        Err(e) => eprintln!("Error, {}", e),
    }

}
#[derive(Debug)]
enum Action {
    SetDP1,
    SetDP2,
    BrightnessUp,
    BrightnessDown,
    LcdOn,
    LcdOff,
    LcdFlash,
    WaitUntil{event_time: SystemTime, next_action: Box<Action>},
    NeverUsed, // avoid a silly rust warning
}


impl Action {
    pub fn future(self, millis: u64) -> Action {
        let now = SystemTime::now();
        println!("now: {:?}", now);
        Action::WaitUntil{event_time: now + Duration::from_millis(millis), next_action: Box::new(self)}
    }
}
const DEBOUNCE_DURATION: Duration = Duration::from_millis(100);

fn process_wait_event(tx: &Sender<Action>, event_time: SystemTime, next_action: Box<Action>) {
    println!("{:?} : {:?}",event_time, SystemTime::now());
    if event_time > SystemTime::now() {
        println!("Delaying more");
        let _ = tx.send(Action::WaitUntil{event_time, next_action});
        return;
    }
    let _ = tx.send(*next_action);
}

fn main() -> Result<(), Box<dyn Error>> {

    let args = Args::parse();
    if args.daemonize == true {
        daemonize();
    }

    let (tx, rx): (Sender<Action>, Receiver<Action>) = mpsc::channel();

    let mut hwconfig = Hw::new();
    
    let mut bl_p = hwconfig.gpio.get(BL)?.into_output();
    let mut key1 = hwconfig.gpio.get(KEY1)?.into_input_pullup();
    let mut key3 = hwconfig.gpio.get(KEY3)?.into_input_pullup();
    let mut joyleft = hwconfig.gpio.get(JOY_LEFT)?.into_input_pullup();
    let mut joyright = hwconfig.gpio.get(JOY_RIGHT)?.into_input_pullup();

    // changing the pull up triggers an edge, which can get caught by the interrupt handlers bellow
    // wait here a bit to let it settle
    thread::sleep(Duration::from_millis(50)); 


    let thread_tx = tx.clone();
    let _ = key1.set_async_interrupt(FallingEdge, move |_| { let _ = thread_tx.send(Action::SetDP1); let _ = thread_tx.send(Action::LcdFlash); thread::sleep(DEBOUNCE_DURATION);})
                .expect("Could not configure Key1");
    
    let thread_tx = tx.clone();
    let _ = key3.set_async_interrupt(FallingEdge, move |_| { let _ = thread_tx.send(Action::SetDP2); let _ = thread_tx.send(Action::LcdFlash); thread::sleep(DEBOUNCE_DURATION);})
                .expect("Could not configure Key3");

    let thread_tx = tx.clone();
    let _ = joyleft.set_async_interrupt(FallingEdge, move |_| { let _ = thread_tx.send(Action::BrightnessDown);thread::sleep(DEBOUNCE_DURATION);})
                .expect("Could not configure JOY LEFT");

    let thread_tx = tx.clone();
    let _ = joyright.set_async_interrupt(FallingEdge, move |_| {let _ = thread_tx.send(Action::BrightnessUp); thread::sleep(DEBOUNCE_DURATION);})
                .expect("Could not configure JOY RIGHT");

    let _ = tx.send(Action::LcdOff);

    println!("Brightness: {:?}",hwconfig.brightness.brightness);
    println!("Current DP: {}", getdp().unwrap());
    println!("Listening for key commands");

    loop {
            let msg = rx.recv().unwrap();
            println!("Message: {:?}", msg);
            match msg {
                Action::SetDP1 => {setdp(DP1); println!("1: {}", getdp().unwrap());}
                Action::SetDP2 => {setdp(DP2); println!("2: {}", getdp().unwrap());}
                Action::BrightnessUp => {hwconfig.brightness.set_brightness('+', 25);}
                Action::BrightnessDown => {hwconfig.brightness.set_brightness('-', 25);}
                Action::LcdFlash => {let _ = tx.send(Action::LcdOn); let _ = tx.send(Action::LcdOff.future(1000));}
                Action::LcdOn => {bl_p.set_high();}
                Action::LcdOff => {bl_p.set_low();}
                Action::WaitUntil{ event_time, next_action} => { process_wait_event(&tx, event_time, next_action); }
                _ =>  {println!("Unhandled message"); continue}
            }
    }
    // never exit
}


