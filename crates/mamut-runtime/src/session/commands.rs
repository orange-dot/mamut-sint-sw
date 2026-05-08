use super::*;

impl RuntimeSession {
    pub fn command_loop(&mut self) -> Result<()> {
        self.print_startup_summary();
        self.print_status()?;

        let stdin = io::stdin();
        let mut lines = stdin.lock().lines();

        loop {
            self.print_runtime_control_messages()?;
            print!("epm1> ");
            io::stdout().flush().context("failed to flush prompt")?;

            let Some(line_result) = lines.next() else {
                println!();
                println!("stdin closed; stopping session");
                break;
            };

            let line = line_result.context("failed to read command input")?;
            match parse_runtime_ui_command(&line) {
                Ok(RuntimeUiCommand::Noop) => continue,
                Ok(RuntimeUiCommand::Help) => print_runtime_help(),
                Ok(RuntimeUiCommand::Status) => self.print_status()?,
                Ok(RuntimeUiCommand::Patches) => list_factory_patches()?,
                Ok(RuntimeUiCommand::Favorites) => list_favorite_patches()?,
                Ok(RuntimeUiCommand::Favorite(slot)) => {
                    self.load_favorite_slot(slot)?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::Patch(argument)) => {
                    let path = resolve_patch_argument(Some(argument.as_str()))?;
                    self.switch_patch(path)?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::NextFavorite) => {
                    self.switch_favorite(1)?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::PrevFavorite) => {
                    self.switch_favorite(-1)?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::DemoPatch) => {
                    self.switch_patch(default_patch_path())?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::Macro(id, value)) => {
                    self.set_macro(id, value)?;
                    println!("macro {} -> {:.3}", macro_display_name(id), value);
                }
                Ok(RuntimeUiCommand::Panic) => {
                    self.panic()?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::ResetControllers) => {
                    self.reset_controllers()?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::BcsLayer(scenario)) => {
                    self.set_bcs_layer_scenario(scenario)?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::Record { seconds, path }) => {
                    let path = self.start_output_recording(seconds, path)?;
                    println!("recording {seconds}s -> {}", path.display());
                    println!(
                        "midi log -> {}",
                        output_recording_midi_log_path(&path).display()
                    );
                }
                Ok(RuntimeUiCommand::RecordStop) => match self.stop_output_recording()? {
                    Some(path) => println!(
                        "recording stop requested -> {} (midi log {})",
                        path.display(),
                        output_recording_midi_log_path(&path).display()
                    ),
                    None => println!("no active output recording"),
                },
                Ok(RuntimeUiCommand::AudioList) => list_audio_devices()?,
                Ok(RuntimeUiCommand::AudioSelect(selector)) => {
                    self.switch_audio(selector)?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::MidiList) => list_midi_devices()?,
                Ok(RuntimeUiCommand::MidiSelect(selector)) => {
                    self.switch_midi(selector)?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::Demo) => {
                    self.enable_demo();
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::Quit) => break,
                Err(error) => eprintln!("command error: {error}"),
            }
        }

        Ok(())
    }

    pub fn block_forever(&mut self) -> Result<()> {
        loop {
            self.print_runtime_control_messages()?;
            thread::sleep(Duration::from_millis(50));
        }
    }

    pub fn switch_patch(&mut self, path: PathBuf) -> Result<()> {
        let keep_demo = self.driver.is_demo();
        let runtime = build_audio_runtime(
            &path,
            self.audio_selector.as_deref(),
            self.sample_rate_hz,
            self.alsa_tuning,
            self.gfm_layer_seed,
            self.bcs_layer_scenario,
            Arc::clone(&self.input_metrics),
            Arc::clone(&self.recording_metrics),
        )?;
        self.install_runtime(path.clone(), runtime, keep_demo)?;
        if self.terminal_output_enabled {
            println!("loaded patch: {} ({})", self.patch_name, path.display());
        }
        Ok(())
    }

    pub fn set_macro(&self, id: MacroId, value: f32) -> Result<()> {
        self.tx
            .send(EngineCommand::Controller(ControllerEvent::Macro {
                id,
                value: value.clamp(0.0, 1.0),
            }))
            .map_err(|_| anyhow!("audio runtime is no longer available"))
    }

    pub fn set_direct_param(&self, id: ParamId, value: f32) -> Result<()> {
        let spec = param_spec(id);
        self.tx
            .send(EngineCommand::Controller(ControllerEvent::DirectParam {
                id,
                value: value.clamp(spec.min, spec.max),
            }))
            .map_err(|_| anyhow!("audio runtime is no longer available"))
    }

    pub fn panic(&self) -> Result<()> {
        self.priority_actions.request_panic();
        Ok(())
    }

    pub fn reset_controllers(&self) -> Result<()> {
        self.priority_actions.request_reset_controllers();
        Ok(())
    }

    pub fn toggle_param(&self, id: ParamId) -> Result<f32> {
        let snapshot = self.request_snapshot()?;
        let current = match id {
            ParamId::ChorusEnabled => snapshot.direct.chorus_enabled,
            ParamId::ReverbEnabled => snapshot.direct.reverb_enabled,
            _ => {
                return Err(anyhow!(
                    "runtime toggle is only supported for chorus_enabled and reverb_enabled"
                ));
            }
        };
        let value = if current { 0.0 } else { 1.0 };
        self.tx
            .send(EngineCommand::Controller(ControllerEvent::DirectParam {
                id,
                value,
            }))
            .map_err(|_| anyhow!("audio runtime is no longer available"))?;
        Ok(value)
    }
}
