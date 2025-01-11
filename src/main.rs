use crate::graphics::math::{add, mul_scalar, Vec2d}; // <1>

use piston_window::*; // <2>

use rand::prelude::*; // <3>

use std::alloc::{GlobalAlloc, Layout, System}; // <4>

use std::time::Instant; // <5>

#[global_allocator] // <6>
static ALLOCATOR: ReportingAllocator = ReportingAllocator;

struct ReportingAllocator; // <7>

unsafe impl GlobalAlloc for ReportingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    let start = Instant::now();
    let ptr = System.alloc(layout);
    let elapsed = start.elapsed();
    
    // Write directly to stderr without allocation
    // libc::write(1, b"Allocation happened in time (in ns): \n".as_ptr() as *const libc::c_void,19);


        // Convert elapsed nanos to a string manually without allocation
        let nanos = elapsed.as_nanos();
        let mut buf = [0u8; 32];  // Static buffer for number conversion
        let mut len = 0;
        let mut n = nanos;
        
        // Handle zero case
        if n == 0 {
            buf[0] = b'0';
            len = 1;
        }
        
        // Convert number to ASCII digits from right to left
        while n > 0 {
            buf[len] = b'0' + (n % 10) as u8;
            n /= 10;
            len += 1;
        }
        
        // Reverse the digits
        for i in 0..len/2 {
            buf.swap(i, len - 1 - i);
        }
        
        // Write prefix
        libc::write(2, b"Allocation took ".as_ptr() as *const _, 16);
        // Write number
        libc::write(2, buf.as_ptr() as *const _, len);
        // Write suffix
        libc::write(2, b" ns\n".as_ptr() as *const _, 4);

    
    ptr
    }



    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }
}

struct World {
    // <9>
    current_turn: u64,             // <9>
    particles: Vec<Box<Particle>>, // <9>
    height: f64,                   // <9>
    width: f64,                    // <9>
    rng: ThreadRng,                // <9>
}

struct Particle {
    // <10>
    height: f64,              // <10>
    width: f64,               // <10>
    position: Vec2d<f64>,     // <10>
    velocity: Vec2d<f64>,     // <10>
    acceleration: Vec2d<f64>, // <10>
    color: [f32; 4],          // <10>
}

impl Particle {
    fn new(world: &World) -> Particle {
        let mut rng = thread_rng();
        let x = rng.gen_range(0.0..=world.width/2.0); // <11>
        let y = world.height/2.0; // <11>
        let x_velocity = rng.gen_range(-0.20..0.20); // <12>
        let y_velocity = rng.gen_range(-0.2..0.20); // <12>
        let x_acceleration = rng.gen_range(-0.15..0.15); // <13>
        let y_acceleration = rng.gen_range(-0.15..0.15); // <13>

        Particle {
            height: rng.gen_range(0.5..=20.0),
            width: rng.gen_range(0.5..=20.0),
            position: [x, y].into(),                               // <14>
            velocity: [x_velocity, y_velocity].into(),             // <14>
            acceleration: [x_acceleration, y_acceleration].into(), // <14>
            color: [rng.gen_range(0.0..=1.0) as f32, rng.gen_range(0.0..=1.0) as f32, rng.gen_range(0.0..=1.0) as f32, rng.gen_range(0.0..=1.0) as f32, ], // <15>
        }
    }

    fn update(&mut self) {
        self.velocity = add(self.velocity, self.acceleration); // <16>
        self.position = add(self.position, self.velocity); // <16>
        self.acceleration = mul_scalar(
            // <17>
            self.acceleration, // <17>
            0.7,               // <17>
        ); // <17>
        self.color[3] *= 0.995; // <18>
    }
}

impl World {
    fn new(width: f64, height: f64) -> World {
        World {
            current_turn: 0,
            particles: Vec::<Box<Particle>>::new(), // <19>
            height: height,
            width: width,
            rng: thread_rng(),
        }
    }

    fn add_shapes(&mut self, n: i32) {
        for _ in 0..n.abs() {
            let particle = Particle::new(&self); // <20>
            let boxed_particle = Box::new(particle); // <21>
            self.particles.push(boxed_particle); // <22>
        }
    }

    fn remove_shapes(&mut self, n: i32) {
        for _ in 0..n.abs() {
            let mut to_delete = None;

            let particle_iter = self
                .particles // <23>
                .iter() // <23>
                .enumerate(); // <23>

            for (i, particle) in particle_iter {
                // <24>
                if particle.color[3] < 0.02 {
                    // <24>
                    to_delete = Some(i); // <24>
                } // <24>
                break; // <24>
            } // <24>
              // <24>
            if let Some(i) = to_delete {
                // <24>
                self.particles.remove(i); // <24>
            } else {
                // <24>
                self.particles.remove(0); // <24>
            }; // <24>
        }
    }

    fn update(&mut self) {
        let n = self.rng.gen_range(-2..=5); // <25>

        if n > 0 {
            self.add_shapes(n);
        } else {
            self.remove_shapes(n);
        }

        self.particles.shrink_to_fit();
        for shape in &mut self.particles {
            shape.update();
        }
        self.current_turn += 1;
    }
}

fn main() {
    let (width, height) = (800.0, 600.0);
    let mut window: PistonWindow = WindowSettings::new("particles", [width, height])
        .exit_on_esc(true)
        .build()
        .expect("Could not create a window.");

    let mut world = World::new(width, height);
    world.add_shapes(10);
    
    while let Some(event) = window.next() {
        world.update();

        window.draw_2d(&event, |ctx, renderer, _device| {
            clear([0.15, 0.17, 0.17, 0.9], renderer);

            for s in &mut world.particles {
                let size = [s.position[0], s.position[1], s.width, s.height];
                rectangle(s.color, size, ctx.transform, renderer);
            }
        });
    }
}
