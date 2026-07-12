use super::*;

impl RuntimeSession {
    pub fn set_gfm_layer_seed(&mut self, seed: Option<u64>) -> Result<GfmVoiceProgramSelection> {
        if self.gfm_layer_seed == seed {
            return Ok(self.request_snapshot()?.gfm_layer.selection);
        }

        let mode = gfm_layer_mode_from_seed(seed);
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(EngineCommand::SetGfmLayerMode(mode, reply_tx))
            .map_err(|_| anyhow!("audio runtime is no longer available"))?;
        let selection = reply_rx
            .recv_timeout(Duration::from_millis(500))
            .map_err(|_| anyhow!("timed out applying GFM layer mode"))?
            .map_err(|error| anyhow!(error))?;

        self.gfm_layer_seed = seed;
        if self.terminal_output_enabled {
            println!(
                "gfm layer {}",
                seed.map(format_gfm_seed)
                    .unwrap_or_else(|| "disabled".to_string())
            );
        }
        Ok(selection)
    }

    pub fn set_bcs_layer_scenario(
        &mut self,
        scenario: Option<BcsScenario>,
    ) -> Result<BcsLayerSnapshot> {
        if self.bcs_layer_scenario == scenario {
            return Ok(self.request_snapshot()?.bcs_layer);
        }

        let mode = bcs_layer_mode_from_scenario(scenario);
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(EngineCommand::SetBcsLayerMode(mode, reply_tx))
            .map_err(|_| anyhow!("audio runtime is no longer available"))?;
        let snapshot = reply_rx
            .recv_timeout(Duration::from_millis(500))
            .map_err(|_| anyhow!("timed out applying BCS layer mode"))?
            .map_err(|error| anyhow!(error))?;

        self.bcs_layer_scenario = scenario;
        if self.terminal_output_enabled {
            println!(
                "bcs layer {}",
                scenario.map(format_bcs_scenario).unwrap_or("disabled")
            );
        }
        Ok(snapshot)
    }

    pub fn set_mozaik_seed(&mut self, seed: Option<u64>) -> Result<MozaikSnapshot> {
        if self.mozaik_seed == seed {
            return Ok(self.request_snapshot()?.mozaik);
        }

        let mode = mozaik_mode_from_seed(seed);
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(EngineCommand::SetMozaikMode(mode, reply_tx))
            .map_err(|_| anyhow!("audio runtime is no longer available"))?;
        let snapshot = reply_rx
            .recv_timeout(Duration::from_millis(500))
            .map_err(|_| anyhow!("timed out applying Mozaik mode"))?
            .map_err(|error| anyhow!(error))?;

        self.mozaik_seed = seed;
        if self.terminal_output_enabled {
            println!(
                "mozaik {}",
                seed.map(format_gfm_seed)
                    .unwrap_or_else(|| "disabled".to_string())
            );
        }
        Ok(snapshot)
    }

    pub fn set_mozaik_param(&self, param: MozaikParam, value: f32) -> Result<MozaikSnapshot> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(EngineCommand::SetMozaikParam(param, value, reply_tx))
            .map_err(|_| anyhow!("audio runtime is no longer available"))?;
        reply_rx
            .recv_timeout(Duration::from_millis(500))
            .map_err(|_| anyhow!("timed out applying Mozaik control"))?
            .map_err(|error| anyhow!(error))
    }

    pub fn switch_audio(&mut self, selector: String) -> Result<()> {
        let keep_demo = self.driver.is_demo();
        let runtime = build_audio_runtime(
            &self.patch_path,
            Some(selector.as_str()),
            self.sample_rate_hz,
            self.alsa_tuning,
            self.gfm_layer_seed,
            self.bcs_layer_scenario,
            self.mozaik_seed,
            Arc::clone(&self.input_metrics),
            Arc::clone(&self.recording_metrics),
        )?;
        self.install_runtime(self.patch_path.clone(), runtime, keep_demo)?;
        if self.terminal_output_enabled {
            println!(
                "audio switched to `{}`",
                self.audio_selector.as_deref().unwrap_or(selector.as_str())
            );
        }
        Ok(())
    }

    pub fn switch_midi(&mut self, selector: String) -> Result<()> {
        if self.midi_selector.as_deref() == Some(selector.as_str())
            && matches!(self.driver, PerformanceDriver::Midi(_))
        {
            if self.terminal_output_enabled {
                println!("midi already switched to `{selector}`");
            }
            return Ok(());
        }

        let mut old_driver = std::mem::replace(&mut self.driver, PerformanceDriver::Idle);
        let old_mode_was_demo = old_driver.is_demo();
        let old_midi_selector = self.midi_selector.clone();
        old_driver.stop();
        drop(old_driver);

        let connection = match open_midi_input(
            Arc::clone(&self.midi_input_queue),
            self.bend_range,
            Some(selector.as_str()),
            self.midi_channel,
            self.controller_profile.clone(),
            self.trace_midi,
            Arc::clone(&self.runtime_control_queue),
            Arc::clone(&self.priority_actions),
            Arc::clone(&self.input_metrics),
            Arc::clone(&self.midi_trace_log),
            self.midi_trace_worker.publisher(),
            Arc::clone(&self.sound_lab_midi_focus),
        ) {
            Ok(Some(connection)) => connection,
            Ok(None) => {
                self.driver = restore_driver_state(
                    self.tx.clone(),
                    Arc::clone(&self.midi_input_queue),
                    self.bend_range,
                    old_midi_selector.as_deref(),
                    self.midi_channel,
                    self.controller_profile.clone(),
                    self.trace_midi,
                    Arc::clone(&self.runtime_control_queue),
                    Arc::clone(&self.priority_actions),
                    Arc::clone(&self.input_metrics),
                    Arc::clone(&self.midi_trace_log),
                    self.midi_trace_worker.publisher(),
                    Arc::clone(&self.sound_lab_midi_focus),
                    old_mode_was_demo,
                );
                return Err(anyhow!("no MIDI input devices available"));
            }
            Err(error) => {
                self.driver = restore_driver_state(
                    self.tx.clone(),
                    Arc::clone(&self.midi_input_queue),
                    self.bend_range,
                    old_midi_selector.as_deref(),
                    self.midi_channel,
                    self.controller_profile.clone(),
                    self.trace_midi,
                    Arc::clone(&self.runtime_control_queue),
                    Arc::clone(&self.priority_actions),
                    Arc::clone(&self.input_metrics),
                    Arc::clone(&self.midi_trace_log),
                    self.midi_trace_worker.publisher(),
                    Arc::clone(&self.sound_lab_midi_focus),
                    old_mode_was_demo,
                );
                return Err(error);
            }
        };

        self.driver = PerformanceDriver::Midi(connection);
        self.midi_selector = Some(selector.clone());
        if self.terminal_output_enabled {
            println!("midi switched to `{selector}`");
        }
        Ok(())
    }

    pub fn enable_demo(&mut self) {
        let mut old_driver = std::mem::replace(&mut self.driver, PerformanceDriver::Idle);
        old_driver.stop();
        self.restore_demo_driver();
        if self.terminal_output_enabled {
            println!("demo performer enabled");
        }
    }

    pub fn switch_favorite(&mut self, direction: isize) -> Result<()> {
        let path = adjacent_live_patch(&self.patch_path, direction)?;
        self.switch_patch(path)
    }

    pub fn load_favorite_slot(&mut self, slot: usize) -> Result<()> {
        let path = live_patch_path(slot)?;
        self.switch_patch(path)
    }
}
