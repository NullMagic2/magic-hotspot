// Module: magic_hotspot.i18n
// Purpose: Live English, Portuguese, Spanish and Greek UI translations.
// Created: 2026-10-13
// Architecture: Adapted from Chop Chop; weak GTK bindings and one saved preference.
use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    Pt,
    Es,
    El,
}
impl Lang {
    pub const ALL: [Self; 4] = [Self::En, Self::Pt, Self::Es, Self::El];
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Pt => "pt",
            Self::Es => "es",
            Self::El => "el",
        }
    }
    pub fn native_name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Pt => "Português",
            Self::Es => "Español",
            Self::El => "Ελληνικά",
        }
    }
    pub fn flag(self) -> &'static [u8] {
        match self {
            Self::En => include_bytes!("../assets/languages/vs-flag-en.svg"),
            Self::Pt => include_bytes!("../assets/languages/vs-flag-pt.svg"),
            Self::Es => include_bytes!("../assets/languages/vs-flag-es.svg"),
            Self::El => include_bytes!("../assets/languages/vs-flag-el.svg"),
        }
    }
    fn from_code(code: &str) -> Option<Self> {
        let code = code
            .split(['_', '-', '.', ':'])
            .next()?
            .trim()
            .to_ascii_lowercase();
        Self::ALL.into_iter().find(|lang| lang.code() == code)
    }
}
thread_local! {
    static LANG:Cell<Lang>=const {Cell::new(Lang::En)};
    static BINDINGS:RefCell<Vec<Box<dyn Fn()->bool>>>=RefCell::new(Vec::new());
}
pub fn lang() -> Lang {
    LANG.with(Cell::get)
}
fn config_file() -> PathBuf {
    glib::user_config_dir()
        .join("magic-hotspot")
        .join("language")
}
fn detect(saved: Option<&str>, locales: impl IntoIterator<Item = String>) -> Lang {
    saved
        .and_then(Lang::from_code)
        .or_else(|| {
            locales
                .into_iter()
                .find_map(|locale| Lang::from_code(&locale))
        })
        .unwrap_or(Lang::En)
}
pub fn init() {
    let saved = std::fs::read_to_string(config_file()).ok();
    let chosen = detect(
        saved.as_deref(),
        glib::language_names()
            .into_iter()
            .map(|locale| locale.to_string()),
    );
    LANG.with(|value| value.set(chosen));
}
fn save(path: &Path, value: Lang) -> Result<(), String> {
    let parent = path.parent().ok_or("invalid language settings directory")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    file.write_all(value.code().as_bytes())
        .map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn set_lang(value: Lang) -> Result<(), String> {
    let result = save(&config_file(), value);
    LANG.with(|lang| lang.set(value));
    BINDINGS.with(|items| items.borrow_mut().retain(|update| update()));
    result
}
pub fn bind(update: impl Fn() -> bool + 'static) {
    if update() {
        BINDINGS.with(|items| items.borrow_mut().push(Box::new(update)));
    }
}
pub fn label(key: &'static str) -> gtk::Label {
    let label = gtk::Label::new(None);
    let weak = label.downgrade();
    bind(move || {
        weak.upgrade()
            .map(|label| label.set_text(&tr(key)))
            .is_some()
    });
    label
}
pub fn bind_button(button: &gtk::Button, key: &'static str) {
    let weak = button.downgrade();
    bind(move || {
        weak.upgrade()
            .map(|button| button.set_label(&tr(key)))
            .is_some()
    });
}
pub fn button(key: &'static str) -> gtk::Button {
    let button = gtk::Button::new();
    bind_button(&button, key);
    button
}
pub fn menu_item(key: &'static str) -> gtk::MenuItem {
    let item = gtk::MenuItem::new();
    let weak = item.downgrade();
    bind(move || {
        weak.upgrade()
            .map(|item| item.set_label(&tr(key)))
            .is_some()
    });
    item
}
pub fn tooltip(widget: &impl IsA<gtk::Widget>, key: &'static str) {
    let weak = widget.upcast_ref::<gtk::Widget>().downgrade();
    bind(move || {
        weak.upgrade()
            .map(|widget| widget.set_tooltip_text(Some(&tr(key))))
            .is_some()
    });
}
pub fn tr(text: &str) -> String {
    translate(lang(), text).to_owned()
}
fn translate(language: Lang, text: &str) -> &str {
    let index = match language {
        Lang::En => return text,
        Lang::Pt => 1,
        Lang::Es => 2,
        Lang::El => 3,
    };
    TABLE
        .iter()
        .find(|row| row[0] == text)
        .map(|row| row[index])
        .unwrap_or(text)
}
pub fn trf(key: &str, values: &[(&str, &str)]) -> String {
    // Substitute only placeholders in the template, never inside a device name.
    let template = tr(key);
    let mut out = String::new();
    let mut remaining = template.as_str();
    while let Some(at) = remaining.find('{') {
        out.push_str(&remaining[..at]);
        remaining = &remaining[at..];
        let Some(end) = remaining.find('}') else {
            out.push_str(remaining);
            return out;
        };
        let token = &remaining[1..end];
        if let Some((_, value)) = values.iter().find(|(name, _)| *name == token) {
            out.push_str(value);
        } else {
            out.push_str(&remaining[..=end]);
        }
        remaining = &remaining[end + 1..];
    }
    out.push_str(remaining);
    out
}
const TABLE: &[[&str; 4]] = &[
    ["Language", "Idioma", "Idioma", "Γλώσσα"],
    [
        "Nearby devices",
        "Dispositivos próximos",
        "Dispositivos cercanos",
        "Κοντινές συσκευές",
    ],
    [
        "Start when I sign in",
        "Iniciar ao entrar",
        "Iniciar al iniciar sesión",
        "Εκκίνηση κατά τη σύνδεση",
    ],
    ["Refresh", "Atualizar", "Actualizar", "Ανανέωση"],
    ["Connect", "Conectar", "Conectar", "Σύνδεση"],
    ["Disconnect", "Desconectar", "Desconectar", "Αποσύνδεση"],
    [
        "Connected ({mode})",
        "Conectado ({mode})",
        "Conectado ({mode})",
        "Συνδεδεμένο ({mode})",
    ],
    ["Connection", "Conexão", "Conexión", "Σύνδεση"],
    [
        "Connecting to {target}…",
        "Conectando a {target}…",
        "Conectando a {target}…",
        "Σύνδεση με {target}…",
    ],
    [
        "Disconnecting…",
        "Desconectando…",
        "Desconectando…",
        "Αποσύνδεση…",
    ],
    [
        "Loading devices",
        "Carregando dispositivos",
        "Cargando dispositivos",
        "Φόρτωση συσκευών",
    ],
    [
        "Loading devices…",
        "Carregando dispositivos…",
        "Cargando dispositivos…",
        "Φόρτωση συσκευών…",
    ],
    [
        "Looking for devices…",
        "Procurando dispositivos…",
        "Buscando dispositivos…",
        "Αναζήτηση συσκευών…",
    ],
    [
        "Connection failed",
        "Falha na conexão",
        "Error de conexión",
        "Αποτυχία σύνδεσης",
    ],
    ["Close", "Fechar", "Cerrar", "Κλείσιμο"],
    ["Open", "Abrir", "Abrir", "Άνοιγμα"],
    ["Quit", "Sair", "Salir", "Έξοδος"],
    ["No", "Não", "No", "Όχι"],
    ["Yes", "Sim", "Sí", "Ναι"],
    [
        "Connect to Wi-Fi",
        "Conectar ao Wi-Fi",
        "Conectar a Wi-Fi",
        "Σύνδεση σε Wi-Fi",
    ],
    [
        "This network does not encrypt your connection. Connect anyway?",
        "Esta rede não criptografa sua conexão. Conectar mesmo assim?",
        "Esta red no cifra tu conexión. ¿Conectar de todos modos?",
        "Αυτό το δίκτυο δεν κρυπτογραφεί τη σύνδεσή σας. Να συνδεθείτε ούτως ή άλλως;",
    ],
    [
        "Open · not encrypted",
        "Aberta · sem criptografia",
        "Abierta · sin cifrado",
        "Ανοικτό · χωρίς κρυπτογράφηση",
    ],
    [
        "Select a saved network to connect.",
        "Selecione uma rede salva para conectar.",
        "Selecciona una red guardada para conectar.",
        "Επιλέξτε ένα αποθηκευμένο δίκτυο για σύνδεση.",
    ],
    [
        "iPhone via USB",
        "iPhone por USB",
        "iPhone por USB",
        "iPhone μέσω USB",
    ],
    [
        "{name} via Wi-Fi",
        "{name} por Wi-Fi",
        "{name} por Wi-Fi",
        "{name} μέσω Wi-Fi",
    ],
    [
        "{name} via Bluetooth",
        "{name} por Bluetooth",
        "{name} por Bluetooth",
        "{name} μέσω Bluetooth",
    ],
    [
        "Could not",
        "Não foi possível",
        "No se pudo",
        "Δεν ήταν δυνατή",
    ],
    [
        "Could not update startup: {error}",
        "Não foi possível atualizar a inicialização: {error}",
        "No se pudo actualizar el inicio: {error}",
        "Δεν ήταν δυνατή η ενημέρωση της εκκίνησης: {error}",
    ],
    [
        "Could not save language: {error}",
        "Não foi possível salvar o idioma: {error}",
        "No se pudo guardar el idioma: {error}",
        "Δεν ήταν δυνατή η αποθήκευση της γλώσσας: {error}",
    ],
    [
        "Could not complete the connection. Try again.",
        "Não foi possível concluir a conexão. Tente novamente.",
        "No se pudo completar la conexión. Inténtalo de nuevo.",
        "Δεν ήταν δυνατή η ολοκλήρωση της σύνδεσης. Δοκιμάστε ξανά.",
    ],
    [
        "Could not disconnect. Try again.",
        "Não foi possível desconectar. Tente novamente.",
        "No se pudo desconectar. Inténtalo de nuevo.",
        "Δεν ήταν δυνατή η αποσύνδεση. Δοκιμάστε ξανά.",
    ],
    [
        "Enable Personal Hotspot. Unlock iPhone and tap Trust.",
        "Ative o Acesso Pessoal. Desbloqueie o iPhone e toque em Confiar.",
        "Activa Compartir Internet. Desbloquea el iPhone y toca Confiar.",
        "Ενεργοποιήστε το Προσωπικό Hotspot. Ξεκλειδώστε το iPhone και πατήστε Εμπιστοσύνη.",
    ],
    [
        "Unlock your iPhone and try again.",
        "Desbloqueie o iPhone e tente novamente.",
        "Desbloquea el iPhone e inténtalo de nuevo.",
        "Ξεκλειδώστε το iPhone και δοκιμάστε ξανά.",
    ],
    [
        "Trust was declined on the iPhone.",
        "A confiança foi recusada no iPhone.",
        "Se rechazó la confianza en el iPhone.",
        "Η εμπιστοσύνη απορρίφθηκε στο iPhone.",
    ],
    [
        "Accept Trust on the iPhone, then try again.",
        "Aceite Confiar no iPhone e tente novamente.",
        "Acepta Confiar en el iPhone e inténtalo de nuevo.",
        "Πατήστε Εμπιστοσύνη στο iPhone και δοκιμάστε ξανά.",
    ],
    [
        "iPhone authenticated, but the USB network link is still inactive.",
        "iPhone autenticado, mas o link de rede USB ainda está inativo.",
        "iPhone autenticado, pero el enlace de red USB sigue inactivo.",
        "Το iPhone επαληθεύτηκε, αλλά η σύνδεση δικτύου USB παραμένει ανενεργή.",
    ],
    [
        crate::pan::RECOVERY_HINT,
        "O Bluetooth conectou, mas a configuração IP do hotspot falhou novamente. Desative e reative o Acesso Pessoal e tente de novo. Se continuar falhando, reinicie o celular.",
        "Bluetooth se conectó, pero la configuración IP del punto de acceso volvió a fallar. Desactiva y activa Compartir Internet e inténtalo de nuevo. Si sigue fallando, reinicia el teléfono.",
        "Το Bluetooth συνδέθηκε, αλλά η ρύθμιση IP του hotspot απέτυχε ξανά. Απενεργοποιήστε και ενεργοποιήστε το Προσωπικό hotspot και δοκιμάστε ξανά. Αν συνεχίζει να αποτυγχάνει, επανεκκινήστε το τηλέφωνο.",
    ],
    [
        "The Bluetooth connection failed or timed out",
        "A conexão Bluetooth falhou ou expirou",
        "La conexión Bluetooth falló o agotó el tiempo de espera",
        "Η σύνδεση Bluetooth απέτυχε ή έληξε ο χρόνος αναμονής",
    ],
    [
        "No Wi-Fi adapter or iPhone USB connection detected.",
        "Nenhum adaptador Wi-Fi ou conexão USB do iPhone detectado.",
        "No se detectó ningún adaptador Wi-Fi ni conexión USB del iPhone.",
        "Δεν εντοπίστηκε προσαρμογέας Wi-Fi ή σύνδεση USB iPhone.",
    ],
];
#[cfg(test)]
#[path = "../tests/i18n/mod.rs"]
mod tests;
