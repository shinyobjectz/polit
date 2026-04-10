use serde::{Deserialize, Serialize};

/// Economic / industrial sector taxonomy used across simulation events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Sector {
    Energy,
    Auto,
    Agriculture,
    Finance,
    Tech,
    Housing,
    Manufacturing,
    Healthcare,
    Retail,
    Defense,
}

/// A simulation event that game systems (law engine, DM tools, NPC actions)
/// can dispatch into the simulation stack. Collected during a game week and
/// flushed for the Dawn Phase tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SimEvent {
    FiscalBill {
        bill_type: String,
        sector: Option<Sector>,
        amount_gdp_pct: f64,
        distributional_target: Option<String>,
    },
    MonetaryPolicy {
        fed_funds_delta: f64,
    },
    EconomyShock {
        shock_type: String,
        magnitude: f64,
        duration_weeks: u32,
    },
    SectorShock {
        sector: Sector,
        region: Option<String>,
        severity: f64,
    },
    Tariff {
        partner: String,
        product: String,
        rate: f64,
    },
    Scandal {
        actor: String,
        severity: f64,
        scandal_type: String,
        media_amplification: f64,
    },
    Protest {
        protest_type: String,
        scale: f64,
        region: String,
        police_response: String,
    },
    MediaCampaign {
        campaign_type: String,
        target_group: String,
        intensity: f64,
        source: String,
    },
    NaturalDisaster {
        disaster_type: String,
        region: String,
        severity: f64,
    },
    Conflict {
        parties: Vec<String>,
        conflict_type: String,
        escalation_level: f64,
    },
    Sanction {
        target_country: String,
        sector: Option<Sector>,
        intensity: f64,
    },
    AllianceShift {
        country: String,
        direction: String,
        domain: String,
    },
}

/// Collects [`SimEvent`]s during a game week and flushes them for the
/// Dawn Phase tick.
#[derive(Debug, Clone, Default)]
pub struct SimEventQueue {
    events: Vec<SimEvent>,
}

impl SimEventQueue {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Enqueue an event for the next Dawn Phase flush.
    pub fn push(&mut self, event: SimEvent) {
        self.events.push(event);
    }

    /// Drain all queued events and return them. The queue is empty afterwards.
    pub fn flush(&mut self) -> Vec<SimEvent> {
        std::mem::take(&mut self.events)
    }

    /// Returns `true` when there are no pending events.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_sector_shock_roundtrips() {
        let event = SimEvent::SectorShock {
            sector: Sector::Energy,
            region: Some("Ohio".into()),
            severity: 0.7,
        };
        let bytes = rmp_serde::to_vec(&event).unwrap();
        let decoded: SimEvent = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(event, decoded);
    }

    #[test]
    fn event_queue_drains_on_flush() {
        let mut queue = SimEventQueue::new();
        queue.push(SimEvent::MonetaryPolicy {
            fed_funds_delta: 0.25,
        });
        queue.push(SimEvent::SectorShock {
            sector: Sector::Auto,
            region: None,
            severity: 0.3,
        });
        let events = queue.flush();
        assert_eq!(events.len(), 2);
        assert!(queue.flush().is_empty());
    }
}
