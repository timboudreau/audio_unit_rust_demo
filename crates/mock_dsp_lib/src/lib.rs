/// A quick and dirty way of doing DSP in rust for demo purposes only - this code
/// liberally uses unsafe, does no bounds checks and is not the way anyone should
/// actually do it.  It exists only to demonstrate a problem with the AudioUnit C
/// code that will *call* it and should not be used for anything else.  It is written
/// this way purely for brevity, **not** sanity.
use std::sync::atomic::{AtomicU32, AtomicU8};

/// Allocates an effect instance and returns a handle (actually just a pointer) to it.
/// It is up to the caller to call `dispose_mock_dsp(handle)` to drop it.  While this
/// effect does not hold state, many do, so this is the pattern.
#[no_mangle]
pub extern "C" fn create_mock_dsp(gain: *mut f32, pan: *mut f32) -> usize {
    let effect = Box::new(MockDspEffect {
        gain: gain.into(),
        pan: pan.into(),
    });
    Box::leak(effect) as *mut MockDspEffect as usize
}

/// Drop a previously allocated dsp processor.
#[no_mangle]
pub extern "C" fn dispose_mock_dsp(handle: usize) {
    assert!(handle != 0);
    let ptr = handle as *mut MockDspEffect;
    unsafe { core::ptr::drop_in_place(ptr) }
}

/// Process a stereo pair of buffers (for demo purposes, we are only supporting stereo)
#[no_mangle]
pub extern "C" fn mock_dsp_process_stereo(
    handle: usize,
    left_input: *const f32,
    right_input: *const f32,
    frames: usize,
    left_output: *mut f32,
    right_output: *mut f32,
) {
    let borrow = unsafe { &mut *(handle as *mut MockDspEffect) };
    borrow.process_buffer_stereo(left_input, right_input, frames, left_output, right_output)
}

/// Holds pointers to the parameters it was initialized with so they can be read (atomically, as
/// they are written) on the fly when processing (and if this were a more advanced demo, could be
/// ramped).  This demo is so simple you could just pass them in to the render method, but, for example,
/// a 16 band compressor can have parameters numbering in the hundreds.
///
/// So the pattern here is we allocate a struct on the Rust side which holds pointers to the
/// parameters, which it will reread on demand, and let the DSP kernel on the C/C++/Objective-C/Swift
/// side be responsible for allocating and deallocating.
struct MockDspEffect {
    gain: AtomicF32,
    pan: AtomicF32,
}

impl MockDspEffect {
    fn process_stereo_frame(&self, left_sample: f32, right_sample: f32) -> (f32, f32) {
        let gain = self.gain.get();
        let mut left = left_sample * gain;
        let mut right = right_sample * gain;

        let pan = self.pan.get();
        if pan != 0.5 {
            let pan_factor = (0.5 - pan).abs() * 2.;
            if pan < 0.5 {
                right *= 1.0 - pan_factor
            } else {
                left *= 1.0 - pan_factor
            }
        }
        (left, right)
    }

    fn process_buffer_stereo(
        &self,
        left_input: *const f32,
        right_input: *const f32,
        frames: usize,
        left_output: *mut f32,
        right_output: *mut f32,
    ) {
        let mut left_input = SampleInput::from(left_input);
        let mut right_input = SampleInput::from(right_input);
        let mut left_output = SampleOutput::from(left_output);
        let mut right_output = SampleOutput::from(right_output);
        for _ in 0..frames {
            let (left, right) = self
                .process_stereo_frame(left_input.read_unchecked(), right_input.read_unchecked());
            left_output.write_unchecked(left);
            right_output.write_unchecked(right)
        }
    }
}

/// A quick and dirty way to store an f32 in a u32 pointer and ensure the same
/// code is used on both sides of the language divide for updating values
#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
struct AtomicF32 {
    ptr: *mut f32,
}

impl From<*mut f32> for AtomicF32 {
    fn from(ptr: *mut f32) -> Self {
        Self { ptr }
    }
}

impl AtomicF32 {
    fn get(&self) -> f32 {
        read_f32_atomic(self.ptr)
    }
}

#[no_mangle]
pub extern "C" fn read_f32_atomic(ptr: *const f32) -> f32 {
    let cast = ptr as *mut AtomicU32;
    let au: &AtomicU32 = unsafe { &*cast };
    f32::from_bits(au.load(std::sync::atomic::Ordering::Acquire))
}

/// Writing is only done from the C++ side, so omit it from AtomicF32
#[no_mangle]
pub extern "C" fn write_f32_atomic(ptr: *mut f32, value: f32) {
    let cast = ptr as *mut AtomicU32;
    let au: &AtomicU32 = unsafe { &*cast };
    au.store(value.to_bits(), std::sync::atomic::Ordering::Relaxed)
}

/// Used for a spinlock in the DSP kernel which works around a race condition
/// in the original AudioUnit template
#[no_mangle]
pub extern "C" fn read_u8_atomic(ptr: *mut u8) -> u8 {
    let cast = ptr as *mut AtomicU8;
    let au: &AtomicU8 = unsafe { &*cast };
    au.load(std::sync::atomic::Ordering::SeqCst)
}

/// Used for a spinlock in the DSP kernel which works around a race condition
/// in the original AudioUnit template
#[no_mangle]
pub extern "C" fn write_u8_atomic(ptr: *mut u8, value: u8) {
    let cast = ptr as *mut AtomicU8;
    let au = unsafe { &*cast };
    au.store(value, std::sync::atomic::Ordering::SeqCst);
}

/// A dumb pointer + offset into an input buffer
#[derive(Debug, Copy, Clone)]
pub struct SampleInput {
    pub pos: usize,
    pub ptr: *const f32,
}

impl From<*const f32> for SampleInput {
    fn from(value: *const f32) -> Self {
        Self { pos: 0, ptr: value }
    }
}

impl SampleInput {
    #[inline(always)]
    pub fn read_unchecked(&mut self) -> f32 {
        let offset = self.pos * size_of::<f32>();
        let result = unsafe {
            let ptr_loc = ((self.ptr as usize) + offset) as *const f32;
            std::ptr::read::<f32>(ptr_loc)
        };
        self.pos += 1;
        result
    }
}

/// A dumb pointer + offset into an output buffer
#[derive(Debug, Clone)]
pub struct SampleOutput {
    pub(crate) pos: usize,
    pub ptr: *mut f32,
}

impl From<*mut f32> for SampleOutput {
    fn from(value: *mut f32) -> Self {
        Self { ptr: value, pos: 0 }
    }
}

impl SampleOutput {
    #[inline(always)]
    pub fn write_unchecked(&mut self, val: f32) {
        let offset = self.pos * size_of::<f32>();
        unsafe {
            let ptr_loc = ((self.ptr as usize) + offset) as *mut f32;
            std::ptr::write::<f32>(ptr_loc, val);
        }
        self.pos += 1;
    }
}
