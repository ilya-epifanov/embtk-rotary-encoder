/*
   Copyright 2019 Ilya Epifanov

   Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
   http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
   http://opensource.org/licenses/MIT>, at your option. This file may not be
   copied, modified, or distributed except according to those terms.
*/
/*!
A helper for handling rotary encoder input, mainly for use on embedded platforms.

```
use embtk_rotary_encoder::RotaryEncoder;

# fn main() {
// The `u16` type will be accepted as a raw position output from your QEI peripheral
let mut enc: RotaryEncoder<u16, _, _> =
// `4i8` below is the number of raw divisions per full physical division.
// `10u32` is the timeout in any kind of ticks you like.
// You'll supply a timestamp every time you receive an even from a peripheral.
    RotaryEncoder::new(4i8, 10u32);

assert_eq!(enc.get_delta(65534u16, 1), 0); // we haven't moved full 4 divisions yet
assert_eq!(enc.get_delta(65532u16, 2), -1); // full 4 divisions down => 1 logical division down

assert_eq!(enc.get_delta(65530u16, 3), 0);
assert_eq!(enc.get_delta(65528u16, 20), 0); // too late, read about timeouts below
# }
```

A note about timeout:

Sometimes you may lose an event from a peripheral or even a peripheral might be buggy.
In this case you'll end up slightly off grid, i.e. you'll see the transition to a next
division not when you feel the tactile feedback from an encoder but somewhere between the positions.

To remedy that, there's a timeout which, on expiry, makes a current position of an encoder a
reference for subsequent moves.

*/
#![no_std]

use core::default::Default;

use num_traits::Num;
use num_traits::WrappingAdd;
use num_traits::WrappingSub;
use num_traits::Bounded;
use num_traits::AsPrimitive;
use num_traits::CheckedSub;
use num_traits::Unsigned;
use num_traits::Signed;

pub struct RotaryEncoder<Pos, Tick, Delta> where
    Pos: Num + WrappingAdd + WrappingSub + Bounded + Copy + PartialOrd + AsPrimitive<Delta> + Default,
    Tick: Unsigned + Bounded + Copy + PartialOrd + CheckedSub + Default,
    Delta: Signed + Copy + AsPrimitive<Pos>,
{
    last_active: Tick,
    last_effective_raw_position: Pos,
    last_real_raw_position: Pos,
    reset_timeout: Tick,
    div: Delta,
}

impl<Pos, Tick, Delta> RotaryEncoder<Pos, Tick, Delta> where
    Pos: Num + WrappingAdd + WrappingSub + Bounded + Copy + PartialOrd + AsPrimitive<Delta> + Default,
    Tick: Unsigned + Bounded + Copy + PartialOrd + CheckedSub + Default,
    Delta: Signed + Copy + AsPrimitive<Pos>,
{
    pub fn new(div: Delta, reset_timeout: Tick) -> Self {
        RotaryEncoder {
            div,
            last_active: Default::default(),
            last_effective_raw_position: Default::default(),
            last_real_raw_position: Default::default(),
            reset_timeout,
        }
    }

    pub fn get_delta(&mut self, raw_position: Pos, ts: Tick) -> Delta where
    {
        if (self.last_active + self.reset_timeout).checked_sub(&ts) == None {
            self.last_effective_raw_position = self.last_real_raw_position;
        }

        let delta: Delta = raw_position.wrapping_sub(&self.last_effective_raw_position).as_();

        let divisions = delta / self.div;
        let remainder = delta % self.div;

        self.last_effective_raw_position = self.last_effective_raw_position.wrapping_add(&(delta - remainder).as_());
        if self.last_real_raw_position != raw_position {
            self.last_active = ts;
            self.last_real_raw_position = raw_position;
        }
        divisions
    }
}

#[cfg(test)]
mod tests {
    use self::super::*;

    #[test]
    fn zero() {
        let mut enc = RotaryEncoder::new(1i8, 10u32);
        assert_eq!(enc.get_delta(0, 1), 0);
    }

    #[test]
    fn increment() {
        let mut enc = RotaryEncoder::new(1i8, 10u32);
        assert_eq!(enc.get_delta(1, 1), 1);
    }

    #[test]
    fn decrement() {
        let mut enc = RotaryEncoder::new(1i8, 10u32);
        assert_eq!(enc.get_delta(-1, 1), -1);
    }

    #[test]
    fn rollover_up() {
        let mut enc = RotaryEncoder::new(1i8, 10u32);

        assert_eq!(enc.get_delta(127, 1), 127);
        assert_eq!(enc.get_delta(-128, 1), 1);
    }

    #[test]
    fn rollover_down() {
        let mut enc = RotaryEncoder::new(1i8, 10u32);

        assert_eq!(enc.get_delta(-128, 1), -128);
        assert_eq!(enc.get_delta(127, 1), -1);
    }

    #[test]
    fn all_the_way_up() {
        let mut enc = RotaryEncoder::new(1i8, 10u32);

        for p in 1..512 {
            assert_eq!(enc.get_delta((p % 256) as i8, 1), 1);
        }
    }

    #[test]
    fn all_the_way_down() {
        let mut enc = RotaryEncoder::new(1i8, 10u32);

        for p in -1..-512 {
            assert_eq!(enc.get_delta((p % 256) as i8, 1), 1);
        }
    }

    #[test]
    fn increment_4divs() {
        let mut enc = RotaryEncoder::new(4i8, 10u32);

        assert_eq!(enc.get_delta(1, 1), 0);
        assert_eq!(enc.get_delta(2, 1), 0);
        assert_eq!(enc.get_delta(3, 1), 0);
        assert_eq!(enc.get_delta(4, 1), 1);
    }

    #[test]
    fn decrement_4divs() {
        let mut enc = RotaryEncoder::new(4i8, 10u32);
        assert_eq!(enc.get_delta(-1, 1), 0);
        assert_eq!(enc.get_delta(-2, 1), 0);
        assert_eq!(enc.get_delta(-3, 1), 0);
        assert_eq!(enc.get_delta(-4, 1), -1);
    }

    #[test]
    fn rollover_up_4divs() {
        let mut enc = RotaryEncoder::new(4i8, 10u32);

        assert_eq!(enc.get_delta(124, 1), 31);
        assert_eq!(enc.get_delta(127, 1), 0);
        assert_eq!(enc.get_delta(-128, 1), 1);
        assert_eq!(enc.get_delta(-127, 1), 0);
        assert_eq!(enc.get_delta(-126, 1), 0);
        assert_eq!(enc.get_delta(-125, 1), 0);
        assert_eq!(enc.get_delta(-124, 1), 1);
    }

    #[test]
    fn rollover_down_4divs() {
        let mut enc = RotaryEncoder::new(4i8, 10u32);

        assert_eq!(enc.get_delta(-128, 1), -32);
        assert_eq!(enc.get_delta(127, 1), 0);
        assert_eq!(enc.get_delta(126, 1), 0);
        assert_eq!(enc.get_delta(125, 1), 0);
        assert_eq!(enc.get_delta(124, 1), -1);
    }

    #[test]
    fn all_the_way_up_4divs() {
        let mut enc = RotaryEncoder::new(4i8, 10u32);

        for p in 1..512 {
            assert_eq!(enc.get_delta((p % 256) as i8, 1), if p % 4 == 0 { 1 } else { 0 });
        }
    }

    #[test]
    fn all_the_way_down_4divs() {
        let mut enc = RotaryEncoder::new(4i8, 10u32);

        for p in -1..-512 {
            assert_eq!(enc.get_delta((p % 256) as i8, 1), if p % 4 == 0 { -1 } else { 0 });
        }
    }

    #[test]
    fn zero_unsigned() {
        let mut enc: RotaryEncoder<u8, _, _> = RotaryEncoder::new(1i8, 10u32);
        assert_eq!(enc.get_delta(0, 1), 0);
    }

    #[test]
    fn increment_unsigned() {
        let mut enc: RotaryEncoder<u8, _, _> = RotaryEncoder::new(1i8, 10u32);
        assert_eq!(enc.get_delta(1, 1), 1);
    }

    #[test]
    fn decrement_unsigned() {
        let mut enc: RotaryEncoder<u8, _, _> = RotaryEncoder::new(1i8, 10u32);
        assert_eq!(enc.get_delta(255, 1), -1);
    }

    #[test]
    fn rollover_up_unsigned() {
        let mut enc: RotaryEncoder<u8, _, _> = RotaryEncoder::new(1i8, 10u32);

        assert_eq!(enc.get_delta(127, 1), 127);
        assert_eq!(enc.get_delta(128, 1), 1);
        assert_eq!(enc.get_delta(255, 1), 127);
        assert_eq!(enc.get_delta(0, 1), 1);
    }

    #[test]
    fn rollover_down_unsigned() {
        let mut enc: RotaryEncoder<u8, _, _> = RotaryEncoder::new(1i8, 10u32);

        assert_eq!(enc.get_delta(255, 1), -1);
    }

    #[test]
    fn all_the_way_up_unsigned() {
        let mut enc: RotaryEncoder<u8, _, _> = RotaryEncoder::new(1i8, 10u32);

        for p in 1..512 {
            assert_eq!(enc.get_delta((p % 256) as u8, 1), 1);
        }
    }

    #[test]
    fn all_the_way_down_unsigned() {
        let mut enc: RotaryEncoder<u8, _, _> = RotaryEncoder::new(1i8, 10u32);

        for p in -1..-512 {
            assert_eq!(enc.get_delta((p % 256) as u8, 1), 1);
        }
    }

    #[test]
    fn increment_4divs_unsigned() {
        let mut enc: RotaryEncoder<u8, _, _> = RotaryEncoder::new(4i8, 10u32);

        assert_eq!(enc.get_delta(1, 1), 0);
        assert_eq!(enc.get_delta(2, 1), 0);
        assert_eq!(enc.get_delta(3, 1), 0);
        assert_eq!(enc.get_delta(4, 1), 1);
    }

    #[test]
    fn decrement_4divs_unsigned() {
        let mut enc: RotaryEncoder<u8, _, _> = RotaryEncoder::new(4i8, 10u32);
        assert_eq!(enc.get_delta(255, 1), 0);
        assert_eq!(enc.get_delta(254, 1), 0);
        assert_eq!(enc.get_delta(253, 1), 0);
        assert_eq!(enc.get_delta(252, 1), -1);
    }

    #[test]
    fn rollover_up_4divs_unsigned() {
        let mut enc: RotaryEncoder<u8, _, _> = RotaryEncoder::new(4i8, 10u32);

        assert_eq!(enc.get_delta(127, 1), 31);
        assert_eq!(enc.get_delta(128, 1), 1);
        assert_eq!(enc.get_delta(252, 1), 31);
        assert_eq!(enc.get_delta(253, 1), 0);
        assert_eq!(enc.get_delta(254, 1), 0);
        assert_eq!(enc.get_delta(255, 1), 0);
        assert_eq!(enc.get_delta(0, 1), 1);
        assert_eq!(enc.get_delta(1, 1), 0);
        assert_eq!(enc.get_delta(2, 1), 0);
        assert_eq!(enc.get_delta(3, 1), 0);
        assert_eq!(enc.get_delta(4, 1), 1);
    }

    #[test]
    fn rollover_down_4divs_unsigned() {
        let mut enc: RotaryEncoder<u8, _, _> = RotaryEncoder::new(4i8, 10u32);

        assert_eq!(enc.get_delta(255, 1), 0);
        assert_eq!(enc.get_delta(254, 1), 0);
        assert_eq!(enc.get_delta(253, 1), 0);
        assert_eq!(enc.get_delta(252, 1), -1);
    }

    #[test]
    fn all_the_way_up_4divs_unsigned() {
        let mut enc: RotaryEncoder<u8, _, _> = RotaryEncoder::new(4i8, 10u32);

        for p in 1..512 {
            assert_eq!(enc.get_delta((p % 256) as u8, 1), if p % 4 == 0 { 1 } else { 0 });
        }
    }

    #[test]
    fn all_the_way_down_4divs_unsigned() {
        let mut enc: RotaryEncoder<u8, _, _> = RotaryEncoder::new(4i8, 10u32);

        for p in -1..-512 {
            assert_eq!(enc.get_delta((p % 256) as u8, 1), if p % 4 == 0 { -1 } else { 0 });
        }
    }
}