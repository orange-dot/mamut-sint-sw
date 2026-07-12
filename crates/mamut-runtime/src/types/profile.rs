use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactoryPatchEntry {
    pub stem: String,
    pub slug: String,
    pub path: PathBuf,
    pub patch_name: String,
    pub description: Option<String>,
    pub favorite: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayOptions {
    pub patch_path: PathBuf,
    pub force_demo: bool,
    pub audio_selector: Option<String>,
    pub sample_rate_hz: u32,
    pub alsa_period_frames: Option<usize>,
    pub alsa_buffer_frames: Option<usize>,
    pub alsa_start_threshold_frames: Option<usize>,
    pub midi_selector: Option<String>,
    pub midi_channel: Option<u8>,
    pub controller_profile_path: Option<PathBuf>,
    pub trace_midi: bool,
    pub headless: bool,
    pub gui: bool,
    pub gfm_layer_seed: Option<u64>,
    pub bcs_layer_scenario: Option<BcsScenario>,
    pub mozaik_seed: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DryRunOptions {
    pub patch_path: PathBuf,
    pub gfm_layer_seed: Option<u64>,
    pub bcs_layer_scenario: Option<BcsScenario>,
    pub mozaik_seed: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlsaOutputDevice {
    pub selector: String,
    pub card_index: i32,
    pub device_index: i32,
    pub card_name: String,
    pub pcm_name: String,
}

impl AlsaOutputDevice {
    pub fn display_name(&self) -> String {
        format!("{} ({}, {})", self.selector, self.card_name, self.pcm_name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlsaPlaybackTuning {
    pub period_frames: usize,
    pub buffer_frames: usize,
    pub start_threshold_frames: usize,
}

impl Default for AlsaPlaybackTuning {
    fn default() -> Self {
        Self {
            period_frames: ALSA_PERIOD_FRAMES_DEFAULT,
            buffer_frames: ALSA_BUFFER_FRAMES_DEFAULT,
            start_threshold_frames: ALSA_START_THRESHOLD_FRAMES_DEFAULT,
        }
    }
}

#[derive(Clone)]
pub struct NamedMidiPort {
    pub port: MidiInputPort,
    pub name: String,
}

#[derive(Debug)]
pub enum EngineCommand {
    Note(NoteEvent),
    Controller(ControllerEvent),
    RequestSnapshot(mpsc::Sender<EngineSnapshot>),
    RequestPatch(mpsc::Sender<PatchFileV1>),
    SetGfmLayerMode(
        GfmLayerMode,
        mpsc::Sender<std::result::Result<GfmVoiceProgramSelection, String>>,
    ),
    SetBcsLayerMode(
        BcsLayerMode,
        mpsc::Sender<std::result::Result<BcsLayerSnapshot, String>>,
    ),
    SetMozaikMode(
        MozaikMode,
        mpsc::Sender<std::result::Result<MozaikSnapshot, String>>,
    ),
    SetMozaikParam(
        MozaikParam,
        f32,
        mpsc::Sender<std::result::Result<MozaikSnapshot, String>>,
    ),
    StartOutputRecording(
        OutputRecordingRequest,
        mpsc::Sender<std::result::Result<PathBuf, String>>,
    ),
    StopOutputRecording(mpsc::Sender<std::result::Result<Option<PathBuf>, String>>),
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeControlMessage {
    ProgramChange(u8),
    Panic,
    ResetControllers,
    NextFavorite,
    PrevFavorite,
    FavoriteSlot(usize),
    ToggleParam(ParamId),
}

#[derive(Debug, Clone)]
pub struct ControllerProfile {
    pub name: String,
    pub path: PathBuf,
    pub bindings_by_cc: HashMap<u8, ControllerBinding>,
}

impl ControllerProfile {
    pub fn binding_for_cc(&self, cc: u8) -> Option<&ControllerBinding> {
        self.bindings_by_cc.get(&cc)
    }
}

#[derive(Debug, Clone)]
pub struct ControllerBinding {
    pub cc: u8,
    pub control: String,
    pub section: ControllerBindingSection,
    pub index: Option<u8>,
    pub action: ControllerBindingAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControllerBindingSection {
    Knob,
    Slider,
    Switch,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControllerBindingAction {
    Macro(MacroId),
    DirectParam {
        id: ParamId,
        scale: ControllerValueScale,
    },
    GfmLayerAmount,
    BcsLayerAmount,
    BcsLayerEnabled,
    Runtime(RuntimeControlMessage),
    ToggleParam(ParamId),
    Reserved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControllerValueScale {
    Linear,
    Log,
}

#[derive(Debug, Deserialize)]
pub struct ControllerProfileFile {
    pub name: Option<String>,
    #[serde(default)]
    pub binding: Vec<ControllerBindingFile>,
}

#[derive(Debug, Deserialize)]
pub struct ControllerBindingFile {
    pub control: String,
    pub cc: u8,
    pub section: Option<ControllerBindingSection>,
    pub index: Option<u8>,
    pub kind: ControllerBindingKind,
    pub target: Option<String>,
    pub action: Option<String>,
    pub slot: Option<usize>,
    pub scale: Option<ControllerValueScale>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControllerBindingKind {
    Macro,
    DirectParam,
    GfmLayerAmount,
    BcsLayerAmount,
    BcsLayerGain,
    BcsLayerEnabled,
    RuntimeAction,
    ToggleParam,
    Reserved,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeUiCommand {
    Noop,
    Help,
    Status,
    Patches,
    Favorites,
    Favorite(usize),
    Patch(String),
    NextFavorite,
    PrevFavorite,
    DemoPatch,
    Macro(MacroId, f32),
    Panic,
    ResetControllers,
    BcsLayer(Option<BcsScenario>),
    MozaikStatus,
    MozaikMode(Option<u64>),
    MozaikSet(MozaikParam, f32),
    Record { seconds: u64, path: Option<PathBuf> },
    RecordStop,
    AudioList,
    AudioSelect(String),
    MidiList,
    MidiSelect(String),
    Demo,
    Quit,
}
