use std::{iter, sync::Arc, time::Duration};

use cpal::{
    BufferSize, Stream,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use glicol_synth::{
    AudioContext, AudioContextBuilder, Message, Sum, oscillator::TriOsc, signal::ConstSig,
};
use parking_lot::Mutex;
use petgraph::graph::NodeIndex;
use ringbuf::{
    HeapRb, SharedRb,
    storage::Heap,
    traits::{Consumer, Observer, Producer, Split},
    wrap::caching::Caching,
};
use tap::Tap;
use tokio::time::sleep;

pub type AudioProducer = Caching<Arc<SharedRb<Heap<f32>>>, true, false>;
pub type AudioConsumer = Caching<Arc<SharedRb<Heap<f32>>>, false, true>;
pub type AudioContextPtr = Arc<Mutex<AudioContext<AUDIO_CONTEXT_BUFFER_SIZE>>>;
const AUDIO_CONTEXT_BUFFER_SIZE: usize = 128;
const AUDIO_RB_SIZE: usize = AUDIO_CONTEXT_BUFFER_SIZE * 4;
pub struct AudioHandle {
    pub context: AudioContextPtr,
    pub stream: Stream,
    pub sum_node: NodeIndex,
    pub volume: Mutex<f32>,
    sample_rate: usize,
}

macro_rules! with_context_lock {
    ($context_expr:expr,$ctx:ident,$code_block:block ) => {{
        let mut $ctx = $context_expr.lock();
        $code_block
    }};
}
#[allow(unused)]
pub(crate) use with_context_lock;

impl AudioHandle {
    pub fn new() -> anyhow::Result<AudioHandle> {
        let rb = HeapRb::<f32>::new(AUDIO_RB_SIZE);
        let (mut prod, mut cons) = rb.split();
        // 获取音频设备
        let host = cpal::default_host();
        let device = host.default_output_device().expect("no output device");
        let supported_config = device.default_output_config()?;
        let mut config = supported_config.config();
        config.buffer_size = BufferSize::Fixed(AUDIO_CONTEXT_BUFFER_SIZE as u32);
        let sr = config.sample_rate as usize;
        let output_channels = config.channels as usize;
        println!(
            "sample rate: {}, channels: {}, buffer size: {:?}",
            config.sample_rate, config.channels, config.buffer_size
        );
        // 创建音频上下文
        let context = AudioContextBuilder::<AUDIO_CONTEXT_BUFFER_SIZE>::new()
            .sr(sr)
            .channels(output_channels)
            .build();

        let context_ptr = Arc::new(Mutex::new(context));
        let context = context_ptr.clone();
        let sum_node: NodeIndex = {
            with_context_lock!(context, ctx, {
                let dst = ctx.destination;
                ctx.add_mono_node(Sum {}).tap(|&n| {
                    ctx.connect(n, dst);
                })
            })
        };

        // 创建音频流
        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // 填充音频数据
                for datum in data.iter_mut() {
                    // 如果缓冲区为空，则从音频上下文获取新的音频块
                    if cons.is_empty() {
                        let buf = context.lock().next_block().to_owned();
                        prod.push_iter(iter::from_coroutine(
                            #[coroutine]
                            || {
                                for frame in 0..buf[0].len() {
                                    for channel in 0..output_channels {
                                        yield buf[channel][frame];
                                    }
                                }
                            },
                        ));
                    }
                    *datum = cons.try_pop().unwrap_or(0.0);
                }
            },
            |err| eprintln!("音频流错误: {}", err),
            None,
        )?;

        stream.play()?;
        Ok(AudioHandle {
            context: context_ptr,
            stream,
            sum_node,
            volume: Mutex::new(0.5),
            sample_rate: sr,
        })
    }

    pub fn set_volume(&self, volume: f32) {
        *self.volume.lock() = volume.clamp(0.0, 1.0);
    }

    pub fn volume(&self) -> f32 {
        *self.volume.lock()
    }

    pub async fn play_note(&self, freq: f32, duration_sec: f32) {
        let volume = self.volume();
        let nodes = with_context_lock!(self.context, ctx, {
            let osc = ctx.add_mono_node(TriOsc::new().freq(freq).sr(self.sample_rate));
            let gate = ctx.add_mono_node(ConstSig::new(1.0));
            let asdr = ctx.add_mono_node(glicol_synth::envelope::Adsr::new());
            let apply_gate = ctx.add_mono_node(glicol_synth::operator::Mul::new(1.0));
            ctx.connect(gate, asdr);
            ctx.connect(osc, apply_gate);
            ctx.connect(asdr, apply_gate);
            let final_mul = ctx.add_mono_node(glicol_synth::operator::Mul::new(volume));
            ctx.connect(apply_gate, final_mul);
            ctx.connect(final_mul, self.sum_node);
            vec![osc, gate, asdr, apply_gate, final_mul]
        });
        sleep(Duration::from_secs_f32(duration_sec)).await;
        with_context_lock!(self.context, ctx, {
            let gate = nodes[1];
            ctx.send_msg(gate, Message::SetToNumber(0, 0.0));
        });
        sleep(Duration::from_secs_f32(0.5)).await;
        with_context_lock!(self.context, ctx, {
            for &node in &nodes {
                ctx.graph.remove_node(node);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use tokio::task::JoinSet;

    use super::*;

    // #[tokio::test]
    #[allow(unused)]
    async fn test_init_audio() {
        let h = Arc::new(AudioHandle::new().unwrap());

        let mut join_set = JoinSet::new();
        for f in &[261.63] {
            let h = Arc::clone(&h);
            join_set.spawn(async move {
                h.play_note(*f, 2.0).await;
            });
        }
        join_set.join_all().await;
    }
}
