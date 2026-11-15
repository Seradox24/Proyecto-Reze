use crate::model::Timeline;
use std::time::Instant;

#[derive(Debug)]
pub struct PlaybackClock {
    sample: Timeline,
    anchor: Instant,
    anchored_position: f64,
}

impl Default for PlaybackClock {
    fn default() -> Self {
        Self {
            sample: Timeline::default(),
            anchor: Instant::now(),
            anchored_position: 0.0,
        }
    }
}

impl PlaybackClock {
    pub fn update(&mut self, timeline: Timeline, now: Instant) {
        let changed = timeline.source_updated != self.sample.source_updated
            || timeline.position_ms != self.sample.position_ms
            || timeline.playing != self.sample.playing
            || timeline.rate != self.sample.rate
            || timeline.reliable != self.sample.reliable;
        if changed {
            let published_position = timeline.position_ms as f64
                + if timeline.playing && timeline.reliable {
                    timeline.age_ms as f64 * timeline.rate
                } else {
                    0.0
                };
            let estimated_position = self.position(now) as f64;
            // Small source jitter must not move the active line back and forth.
            self.anchored_position = if timeline.playing
                && self.sample.playing
                && timeline.reliable
                && self.sample.reliable
                && timeline.rate == self.sample.rate
                && (published_position - estimated_position).abs() < 150.0
            {
                estimated_position
            } else {
                published_position
            };
            self.anchor = now;
        }
        self.sample = timeline;
    }
    pub fn position(&self, now: Instant) -> u64 {
        let elapsed = if self.sample.playing && self.sample.reliable {
            now.saturating_duration_since(self.anchor).as_secs_f64() * 1000.0 * self.sample.rate
        } else {
            0.0
        };
        let position = (self.anchored_position + elapsed).max(0.0) as u64;
        self.sample
            .duration_ms
            .map_or(position, |duration| position.min(duration))
    }
    pub fn reliable(&self) -> bool {
        self.sample.reliable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[test]
    fn small_jitter_is_ignored_but_real_seek_reanchors() {
        let now = Instant::now();
        let mut clock = PlaybackClock::default();
        let sample = Timeline {
            position_ms: 1000,
            source_updated: 1,
            playing: true,
            rate: 1.0,
            reliable: true,
            ..Default::default()
        };
        clock.update(sample.clone(), now);
        clock.update(
            Timeline {
                position_ms: 1950,
                source_updated: 2,
                ..sample.clone()
            },
            now + Duration::from_secs(1),
        );
        assert_eq!(clock.position(now + Duration::from_secs(1)), 2000);
        clock.update(
            Timeline {
                position_ms: 500,
                source_updated: 3,
                ..sample
            },
            now + Duration::from_secs(2),
        );
        assert_eq!(clock.position(now + Duration::from_secs(2)), 500);
    }
    #[test]
    fn repeated_source_sample_does_not_reset_clock_and_pause_freezes() {
        let now = Instant::now();
        let mut clock = PlaybackClock::default();
        let mut t = Timeline {
            position_ms: 1000,
            playing: true,
            rate: 1.0,
            reliable: true,
            source_updated: 1,
            ..Default::default()
        };
        clock.update(t.clone(), now);
        t.age_ms = 500;
        clock.update(t.clone(), now + Duration::from_millis(500));
        assert_eq!(clock.position(now + Duration::from_secs(1)), 2000);
        t.playing = false;
        t.position_ms = 2000;
        clock.update(t, now + Duration::from_secs(1));
        assert_eq!(clock.position(now + Duration::from_secs(8)), 2000);
    }
    #[test]
    fn seek_rate_duration_and_unreliable() {
        let now = Instant::now();
        let mut clock = PlaybackClock::default();
        clock.update(
            Timeline {
                position_ms: 4000,
                age_ms: 500,
                duration_ms: Some(6000),
                playing: true,
                rate: 2.0,
                reliable: true,
                source_updated: 2,
            },
            now,
        );
        assert_eq!(clock.position(now), 5000);
        assert_eq!(clock.position(now + Duration::from_secs(1)), 6000);
        clock.update(
            Timeline {
                position_ms: 100,
                reliable: false,
                playing: true,
                rate: 1.0,
                ..Default::default()
            },
            now,
        );
        assert_eq!(clock.position(now + Duration::from_secs(1)), 100);
    }
}
