use rusteal_ffi::{RustealSlateProps, RustealSlateSlot};

use crate::delegate_registry::register_callback;
use crate::error::{RustealError, RustealResult, check_ffi};
use crate::ffi_dispatch;

const TEXT_BLOCK: u32 = 1;
const BUTTON: u32 = 2;
const CHECK_BOX: u32 = 3;
const EDITABLE_TEXT_BOX: u32 = 4;
const SPIN_BOX: u32 = 5;
const VERTICAL_BOX: u32 = 6;
const HORIZONTAL_BOX: u32 = 7;
const SCROLL_BOX: u32 = 8;
const BORDER: u32 = 9;
const SPACER: u32 = 10;

/// A Slate widget built from Rust. Dropping it lets go of Rust's hold: a widget placed in a
/// parent or a tab lives as long as they do.
#[derive(Debug)]
pub struct Widget {
    id: u64,
}

impl Drop for Widget {
    fn drop(&mut self) {
        unsafe { ffi_dispatch::widget_slate_release(self.id) };
    }
}

impl Widget {
    /// The text a text block or a text box shows now.
    pub fn text(&self) -> RustealResult<String> {
        let mut len = 0u32;
        check_ffi(unsafe {
            ffi_dispatch::widget_slate_get_text(self.id, std::ptr::null_mut(), 0, &mut len)
        })?;

        let mut buffer = vec![0u8; len as usize];
        check_ffi(unsafe {
            ffi_dispatch::widget_slate_get_text(self.id, buffer.as_mut_ptr(), len, &mut len)
        })?;

        Ok(String::from_utf8_lossy(&buffer).into_owned())
    }

    /// Clicks a button, as the mouse would.
    pub fn simulate_click(&self) -> bool {
        unsafe { ffi_dispatch::widget_slate_simulate_click(self.id) }
    }

    pub(crate) fn from_id(id: u64) -> Option<Self> {
        (id != 0).then_some(Widget { id })
    }

    pub(crate) fn into_id(self) -> u64 {
        let id = self.id;
        std::mem::forget(self);
        id
    }
}

/// A value a widget shows: fixed, or read again from a closure every frame.
pub enum Attribute<T> {
    Fixed(T),
    Bound(Box<dyn FnMut() -> T + Send>),
}

impl From<&str> for Attribute<String> {
    fn from(value: &str) -> Self {
        Attribute::Fixed(value.to_string())
    }
}

impl From<String> for Attribute<String> {
    fn from(value: String) -> Self {
        Attribute::Fixed(value)
    }
}

impl<F: FnMut() -> String + Send + 'static> From<F> for Attribute<String> {
    fn from(f: F) -> Self {
        Attribute::Bound(Box::new(f))
    }
}

impl From<bool> for Attribute<bool> {
    fn from(value: bool) -> Self {
        Attribute::Fixed(value)
    }
}

/// `Attribute::from_fn(|| ...)` for a value other than text read every frame.
impl<T> Attribute<T> {
    pub fn from_fn(f: impl FnMut() -> T + Send + 'static) -> Self {
        Attribute::Bound(Box::new(f))
    }
}

fn text_callback(mut f: Box<dyn FnMut() -> String + Send>) -> u64 {
    register_callback(move |out| {
        let text = f();
        unsafe { ffi_dispatch::widget_slate_write_text(out, text.as_ptr(), text.len() as u32) };
    })
}

fn bool_callback(mut f: Box<dyn FnMut() -> bool + Send>) -> u64 {
    register_callback(move |out| {
        if !out.is_null() {
            unsafe { *(out as *mut bool) = f() };
        }
    })
}

fn read_text_arg(params: *mut u8) -> String {
    if params.is_null() {
        return String::new();
    }

    #[repr(C)]
    struct TextArg {
        utf8: *const u8,
        len: u32,
    }

    let arg = unsafe { &*(params as *const TextArg) };

    if arg.utf8.is_null() {
        return String::new();
    }

    let bytes = unsafe { std::slice::from_raw_parts(arg.utf8, arg.len as usize) };
    String::from_utf8_lossy(bytes).into_owned()
}

#[derive(Default)]
struct Props {
    text: Option<Attribute<String>>,
    hint: String,
    enabled: Option<Attribute<bool>>,
    on_event: u64,
    on_commit: u64,
    value: f32,
    min: f32,
    max: f32,
    checked: bool,
    padding: [f32; 4],
}

impl Props {
    fn create(self, kind: u32) -> RustealResult<Widget> {
        let (text, text_callback) = match self.text {
            Some(Attribute::Fixed(text)) => (text, 0),
            Some(Attribute::Bound(f)) => (String::new(), text_callback(f)),
            None => (String::new(), 0),
        };

        let enabled_callback = match self.enabled {
            Some(Attribute::Bound(f)) => bool_callback(f),
            Some(Attribute::Fixed(enabled)) => bool_callback(Box::new(move || enabled)),
            None => 0,
        };

        let props = RustealSlateProps {
            text: text.as_ptr(),
            text_len: text.len() as u32,
            text_callback,
            hint: self.hint.as_ptr(),
            hint_len: self.hint.len() as u32,
            enabled_callback,
            on_event: self.on_event,
            on_commit: self.on_commit,
            value: self.value,
            min: self.min,
            max: self.max,
            checked: self.checked,
            padding: self.padding,
        };

        let id = unsafe { ffi_dispatch::widget_slate_new(kind, &props) };

        if id == 0 {
            Err(RustealError::invalid_operation(
                "slate: the widget was not created: Slate is not running".to_string(),
            ))
        } else {
            Ok(Widget { id })
        }
    }
}

/// How a child sits in its parent's slot.
#[derive(Debug, Clone, Copy)]
pub struct Slot {
    pub padding: [f32; 4],
    pub h_align: Align,
    pub v_align: Align,
    /// The share of the box's free space the child takes; zero is its own size.
    pub fill: f32,
}

impl Default for Slot {
    fn default() -> Self {
        Slot {
            padding: [0.0; 4],
            h_align: Align::Fill,
            v_align: Align::Fill,
            fill: 0.0,
        }
    }
}

impl Slot {
    pub fn padding(mut self, all: f32) -> Self {
        self.padding = [all; 4];
        self
    }

    pub fn fill(mut self, share: f32) -> Self {
        self.fill = share;
        self
    }

    pub fn h_align(mut self, align: Align) -> Self {
        self.h_align = align;
        self
    }

    pub fn v_align(mut self, align: Align) -> Self {
        self.v_align = align;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Fill = 0,
    Start = 1,
    Center = 2,
    End = 3,
}

fn add_child(parent: &Widget, child: Widget, slot: Slot) -> RustealResult<()> {
    let slot = RustealSlateSlot {
        padding: slot.padding,
        h_align: slot.h_align as u8,
        v_align: slot.v_align as u8,
        fill: slot.fill,
    };

    check_ffi(unsafe { ffi_dispatch::widget_slate_add_child(parent.id, child.id, &slot) })
}

/// Something that becomes a widget: a widget, or a builder of one.
pub trait IntoWidget {
    fn into_widget(self) -> RustealResult<Widget>;
}

impl IntoWidget for Widget {
    fn into_widget(self) -> RustealResult<Widget> {
        Ok(self)
    }
}

impl IntoWidget for RustealResult<Widget> {
    fn into_widget(self) -> RustealResult<Widget> {
        self
    }
}

macro_rules! builder_into_widget {
    ($($builder:ty),*) => {
        $(
            impl IntoWidget for $builder {
                fn into_widget(self) -> RustealResult<Widget> {
                    self.build()
                }
            }
        )*
    };
}

#[derive(Default)]
pub struct TextBlock {
    props: Props,
}

impl TextBlock {
    pub fn new(text: impl Into<Attribute<String>>) -> Self {
        TextBlock {
            props: Props {
                text: Some(text.into()),
                ..Props::default()
            },
        }
    }

    pub fn build(self) -> RustealResult<Widget> {
        self.props.create(TEXT_BLOCK)
    }
}

#[derive(Default)]
pub struct Button {
    props: Props,
    content: Option<RustealResult<Widget>>,
}

impl Button {
    pub fn new() -> Self {
        Button::default()
    }

    pub fn text(mut self, text: impl Into<Attribute<String>>) -> Self {
        self.props.text = Some(text.into());
        self
    }

    pub fn content(mut self, content: impl IntoWidget) -> Self {
        self.content = Some(content.into_widget());
        self
    }

    pub fn enabled(mut self, enabled: impl Into<Attribute<bool>>) -> Self {
        self.props.enabled = Some(enabled.into());
        self
    }

    pub fn on_clicked(mut self, mut f: impl FnMut() + Send + 'static) -> Self {
        self.props.on_event = register_callback(move |_| f());
        self
    }

    pub fn build(self) -> RustealResult<Widget> {
        let button = self.props.create(BUTTON)?;

        if let Some(content) = self.content {
            add_child(&button, content?, Slot::default())?;
        }

        Ok(button)
    }
}

#[derive(Default)]
pub struct CheckBox {
    props: Props,
}

impl CheckBox {
    pub fn new(checked: bool) -> Self {
        CheckBox {
            props: Props {
                checked,
                ..Props::default()
            },
        }
    }

    pub fn on_changed(mut self, mut f: impl FnMut(bool) + Send + 'static) -> Self {
        self.props.on_event = register_callback(move |params| {
            if !params.is_null() {
                f(unsafe { *(params as *const bool) });
            }
        });
        self
    }

    pub fn enabled(mut self, enabled: impl Into<Attribute<bool>>) -> Self {
        self.props.enabled = Some(enabled.into());
        self
    }

    pub fn build(self) -> RustealResult<Widget> {
        self.props.create(CHECK_BOX)
    }
}

#[derive(Default)]
pub struct EditableTextBox {
    props: Props,
}

impl EditableTextBox {
    pub fn new(text: &str) -> Self {
        EditableTextBox {
            props: Props {
                text: Some(Attribute::Fixed(text.to_string())),
                ..Props::default()
            },
        }
    }

    pub fn hint(mut self, hint: &str) -> Self {
        self.props.hint = hint.to_string();
        self
    }

    /// Called on every edit, with the whole text.
    pub fn on_changed(mut self, mut f: impl FnMut(String) + Send + 'static) -> Self {
        self.props.on_event = register_callback(move |params| f(read_text_arg(params)));
        self
    }

    /// Called when the text is committed: Enter, or the focus leaving the box.
    pub fn on_committed(mut self, mut f: impl FnMut(String) + Send + 'static) -> Self {
        self.props.on_commit = register_callback(move |params| f(read_text_arg(params)));
        self
    }

    pub fn build(self) -> RustealResult<Widget> {
        self.props.create(EDITABLE_TEXT_BOX)
    }
}

#[derive(Default)]
pub struct SpinBox {
    props: Props,
}

impl SpinBox {
    pub fn new(value: f32, min: f32, max: f32) -> Self {
        SpinBox {
            props: Props {
                value,
                min,
                max,
                ..Props::default()
            },
        }
    }

    pub fn on_changed(mut self, mut f: impl FnMut(f32) + Send + 'static) -> Self {
        self.props.on_event = register_callback(move |params| {
            if !params.is_null() {
                f(unsafe { *(params as *const f32) });
            }
        });
        self
    }

    pub fn build(self) -> RustealResult<Widget> {
        self.props.create(SPIN_BOX)
    }
}

/// A box of children, top to bottom ([`VerticalBox`]), left to right ([`HorizontalBox`]), or
/// scrolling ([`ScrollBox`]).
pub struct Panel {
    kind: u32,
    slots: Vec<(RustealResult<Widget>, Slot)>,
}

impl Panel {
    pub fn slot(mut self, child: impl IntoWidget) -> Self {
        self.slots.push((child.into_widget(), Slot::default()));
        self
    }

    pub fn slot_with(mut self, slot: Slot, child: impl IntoWidget) -> Self {
        self.slots.push((child.into_widget(), slot));
        self
    }

    pub fn build(self) -> RustealResult<Widget> {
        let panel = Props::default().create(self.kind)?;

        for (child, slot) in self.slots {
            add_child(&panel, child?, slot)?;
        }

        Ok(panel)
    }
}

pub struct VerticalBox;
pub struct HorizontalBox;
pub struct ScrollBox;

impl VerticalBox {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Panel {
        Panel {
            kind: VERTICAL_BOX,
            slots: Vec::new(),
        }
    }
}

impl HorizontalBox {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Panel {
        Panel {
            kind: HORIZONTAL_BOX,
            slots: Vec::new(),
        }
    }
}

impl ScrollBox {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Panel {
        Panel {
            kind: SCROLL_BOX,
            slots: Vec::new(),
        }
    }
}

pub struct Border {
    padding: [f32; 4],
    content: Option<RustealResult<Widget>>,
}

impl Border {
    pub fn new(content: impl IntoWidget) -> Self {
        Border {
            padding: [4.0; 4],
            content: Some(content.into_widget()),
        }
    }

    pub fn padding(mut self, all: f32) -> Self {
        self.padding = [all; 4];
        self
    }

    pub fn build(self) -> RustealResult<Widget> {
        let border = Props {
            padding: self.padding,
            ..Props::default()
        }
        .create(BORDER)?;

        if let Some(content) = self.content {
            add_child(&border, content?, Slot::default())?;
        }

        Ok(border)
    }
}

/// Empty space of a size.
pub fn spacer(width: f32, height: f32) -> RustealResult<Widget> {
    Props {
        min: width,
        max: height,
        ..Props::default()
    }
    .create(SPACER)
}

builder_into_widget!(
    TextBlock,
    Button,
    CheckBox,
    EditableTextBox,
    SpinBox,
    Panel,
    Border
);

/// Registers a tab the editor's **Window** layout can open, whose content `build` makes each
/// time it opens; registering a name again replaces it. A library's tabs close and go away when
/// it unloads: register them from `rusteal_runtime::on_load!`, so a hot reload brings them back.
#[track_caller]
pub fn register_tab(
    name: &str,
    label: &str,
    mut build: impl FnMut() -> RustealResult<Widget> + Send + 'static,
) -> RustealResult<()> {
    let id = register_callback(move |out| {
        let built = build();

        match built {
            Ok(widget) if !out.is_null() => unsafe { *(out as *mut u64) = widget.into_id() },
            Ok(_) => {}
            Err(e) => e.__log_in(crate::LOG_WARNING, "building a tab"),
        }
    });

    check_ffi(unsafe {
        ffi_dispatch::widget_slate_register_tab(
            name.as_ptr(),
            name.len() as u32,
            label.as_ptr(),
            label.len() as u32,
            id,
        )
    })
}

/// Opens the tab registered under `name`, or brings it to front; `false` when there is none.
pub fn open_tab(name: &str) -> bool {
    unsafe { ffi_dispatch::widget_slate_invoke_tab(name.as_ptr(), name.len() as u32) }
}

pub fn unregister_tab(name: &str) {
    unsafe { ffi_dispatch::widget_slate_unregister_tab(name.as_ptr(), name.len() as u32) };
}

#[doc(hidden)]
pub fn live_widgets() -> u32 {
    unsafe { ffi_dispatch::widget_slate_live_widgets() }
}

#[doc(hidden)]
pub fn live_callbacks() -> u32 {
    unsafe { ffi_dispatch::widget_slate_live_callbacks() }
}
