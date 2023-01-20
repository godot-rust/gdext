/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::time::{Duration, Instant};

use std::io::Write;

pub struct StopWatch {
    last_instant: Instant,
    metrics: Vec<Metric>,
    lwidth: usize,
}

impl StopWatch {
    pub fn start() -> Self {
        Self {
            last_instant: Instant::now(),
            metrics: vec![],
            lwidth: 0,
        }
    }

    pub fn record(&mut self, what: &'static str) {
        let now = Instant::now();
        let duration = now - self.last_instant;
        self.last_instant = now;
        self.lwidth = usize::max(self.lwidth, what.len());
        self.metrics.push(Metric {
            name: what,
            duration,
        });
    }

    pub fn write_stats_to(self, to_file: &Path) {
        let file = File::create(to_file).expect("failed to create stats file");
        let mut writer = BufWriter::new(file);

        // Accumulate total
        let mut total = Duration::ZERO;
        for metric in self.metrics.iter() {
            total += metric.duration;
        }
        let rwidth = log10(total.as_millis());
        let total_metric = Metric {
            name: "total",
            duration: total,
        };

        // Write to file
        for metric in self.metrics.iter() {
            Self::write_metric(&mut writer, metric, self.lwidth, rwidth);
        }
        writeln!(&mut writer, "{}", "-".repeat(self.lwidth + rwidth + 5))
            .expect("failed to write to stats file");
        Self::write_metric(&mut writer, &total_metric, self.lwidth, rwidth);
    }

    fn write_metric(writer: &mut BufWriter<File>, metric: &Metric, lwidth: usize, rwidth: usize) {
        writeln!(
            writer,
            "{: >l$}: {: >r$} ms",
            metric.name,
            metric.duration.as_millis(),
            l = lwidth,
            r = rwidth,
        )
        .expect("failed to write to stats file");
    }
}

fn log10(n: u128) -> usize {
    std::iter::successors(Some(n), |&n| (n >= 10).then_some(n / 10)).count()
}

struct Metric {
    name: &'static str,
    duration: Duration,
}
