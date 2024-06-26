use std::{cmp::Ordering, io};

use rand::Rng;
fn main() {
    let secret_number: u32 = rand::thread_rng().gen_range(1..100);
    loop {
        println!("Guess a num and input");
        let mut guess: String = String::new();
        let res: Result<usize, io::Error> = io::stdin().read_line(&mut guess);
        let mut v = vec![12];
        v.push(11);

        match res {
            Ok(size) => {
                println!("read {} bytes", size);
                println!("so, {} is your guess", guess.trim());
            }
            Err(err) => {
                println!("error was encountered {}", err.to_string());
                return;
            }
        }

        let guess_result: Result<u32, std::num::ParseIntError> = guess.trim().parse();

        let guess: u32;
        match guess_result {
            Ok(_guess) => guess = _guess,
            Err(err) => {
                println!("error encountered {}", err.to_string());
                return;
            }
        }

        match guess.cmp(&secret_number) {
            Ordering::Equal => {
                println!("you guessed right");
                break;
            }
            _ => {
                println!(
                    "incorrect guess: {} for secret_number: {}",
                    guess, secret_number
                );
            }
        }
    }
}