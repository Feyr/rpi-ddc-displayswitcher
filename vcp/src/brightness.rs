pub mod brightness {
    pub struct BrightnessHw {}

    pub struct Brightness {
        brightness_hw: BrightnessHw,
        pub brightness: i16,
    }

    #[cfg(target_arch = "arm")]
    impl BrightnessHw {
        pub fn new() -> BrightnessHw {
            BrightnessHw{}
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
                    
            Ok(val)    
        }
        pub fn set(&mut self, val: i16) -> i16 {
            let new_val = min(max(val, 0), 100);

            let mut cmd = Command::new("ddcutil");
            let cmd = cmd.args(["setvcp", "10", &new_val.to_string()]);

            
            let output = cmd.output()
                                    .expect("failed to execute process");
            println!("Setting brightness {:?}: {}: {}", cmd.get_args(), output.status, String::from_utf8(output.stdout).unwrap() );
            new_val
        }


    }

    #[cfg(target_arch = "x86_64")]
    impl BrightnessHw {
        pub fn new() -> BrightnessHw {
            BrightnessHw{}
        }

        pub fn get_brightness(&mut self) -> Result<i16, Box<dyn Error>> {
            Ok(50)    
        }
        pub fn set(&mut self, val: i16) -> i16 {
            let new_val = min(max(val, 0), 100);

            println!("Setting brightness {}", new_val );
            new_val
        }


    }

    impl Brightness {
        pub fn new() -> Brightness {

            let mut b = Brightness {
                brightness_hw: BrightnessHw::new(),
                brightness: 50
            };
            b.brightness = b.brightness_hw.get_brightness().unwrap();
            b
        }

        
        pub fn increase(&mut self, val: i16) {
            let new_brightness = min(self.brightness + val, 100);
            self.set(new_brightness);
        }

        pub fn decrease(&mut self, val: i16) {
            let new_brightness = max(self.brightness - val, 0);
            self.set(new_brightness);
        }

        pub fn set(&mut self, val: i16) {
            self.brightness = self.brightness_hw.set(new_brightness);
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
            assert_eq!(brightness.increase(50), 100);
            brightness.brightness = 100;
            assert_eq!(brightness.increase(50), 100);
            brightness.brightness = 100;
            assert_eq!(brightness.decrease(50), 50);
            brightness.brightness = 50;
            assert_eq!(brightness.decrease(50), 0);
            brightness.brightness = 0;
            assert_eq!(brightness.decrease(50), 0);
            

        }


    }
}