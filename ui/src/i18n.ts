// UI strings in English and French. `fr` must define every key of `en`
// (the type checker enforces it). `{name}` placeholders are filled by t().

const en = {
  "app.title": "Sinden Driver",
  "app.language": "Language",
  "lang.auto": "System",
  "lang.en": "English",
  "lang.fr": "Français",

  "service.connected": "Driver {version} connected",
  "service.disconnected": "Driver not reachable",
  "service.disconnectedHelp":
    "The sinden-driver service is not running or this user cannot reach {endpoint}. Settings can be edited once it is reachable; retrying…",
  "service.apiMismatch": "This UI speaks control API {expected}; the driver speaks {actual}. Some settings may not apply.",

  "guns.title": "Guns",
  "guns.none": "No gun detected",
  "gun.player": "Player {player}",
  "gun.firmware": "firmware {firmware}",
  "state.starting": "starting",
  "state.aiming": "aiming",
  "state.aimed_away": "aimed away",
  "state.calibrating": "calibrating",
  "state.stopped": "stopped",
  "state.unknown": "unknown",

  "tab.player": "Player {player}",
  "tab.general": "General",
  "player2.inherit": "Player 2 settings not set in the file use player 1's value.",

  "actions.save": "Save",
  "actions.revert": "Revert",
  "actions.pending": "{count} unsaved change(s)",
  "actions.saved": "Saved. The driver restarted the guns.",
  "actions.invalid": "Fix the highlighted fields before saving.",
  "actions.error": "The driver refused the change: {error}",

  "general.autostart": "Start the tray application at login",
  "general.configFile": "Configuration file",
  "general.endpoint": "Control endpoint",
  "general.about": "sinden-ui is free software (MIT). The sinden-driver binary has its own license.",

  "section.mode": "Mode and handedness",
  "section.buttons": "Buttons",
  "section.buttonsMouse": "Buttons in mouse and keyboard mode",
  "section.buttonsJoystick": "Buttons in joystick mode",
  "section.recoil": "Recoil",
  "section.camera": "Camera",
  "section.aim": "Offsets and calibration",
  "section.vision": "Image analysis",
  "section.devices": "Devices",
  "section.logging": "Logging",

  "field.Joystick": "Mode",
  "field.LeftHanded": "Handedness",
  "field.GangstaSetting": "Sideways grip",
  "field.ResourceMode": "Resource mode",
  "field.Smooth": "Anti-jitter (smoothing)",
  "field.OffscreenReload": "Offscreen reload",
  "field.IAgreeRecoilTermsInLicense": "I agree to the recoil terms of the license",
  "field.EnableRecoil": "Enable recoil at start-up",
  "field.RecoilStrength": "Recoil strength",
  "field.RecoilTrigger": "Recoil on trigger",
  "field.RecoilTriggerOffscreen": "Recoil on trigger off screen",
  "field.RecoilPumpActionOnEvent": "Recoil on pump forward",
  "field.RecoilPumpActionOffEvent": "Recoil on pump back",
  "field.RecoilFrontLeft": "Recoil on front left button",
  "field.RecoilFrontRight": "Recoil on front right button",
  "field.RecoilBackLeft": "Recoil on rear left button",
  "field.RecoilBackRight": "Recoil on rear right button",
  "field.TriggerRecoilNormalOrRepeat": "Trigger recoil",
  "field.AutoRecoilStrength": "Automatic recoil strength",
  "field.AutoRecoilStartDelay": "Automatic recoil start delay",
  "field.AutoRecoilDelayBetweenPulses": "Delay between automatic pulses",
  "field.CameraExposureAuto": "Exposure",
  "field.CameraExposure": "Exposure time",
  "field.CameraBrightness": "Brightness",
  "field.CameraContrast": "Contrast",
  "field.CameraRes": "Camera resolution",
  "field.OffsetX": "Horizontal offset (%)",
  "field.OffsetY": "Vertical offset (%)",
  "field.OffsetXRatio": "Horizontal factor",
  "field.OffsetYRatio": "Vertical factor",
  "field.CalibrateX": "Calibration X",
  "field.CalibrateY": "Calibration Y",
  "field.EnableCalibration": "Allow calibration mode",
  "field.AutoSaveCalibrationInLightgun": "Save the calibration in the gun",
  "field.ColourMatchRGB": "Border colour",
  "field.ColourMatchRange": "Colour tolerance",
  "field.MatchOnlyWherePointing": "Match only where pointing",
  "field.SerialPortWrite": "Serial port",
  "field.SerialPortSecondary": "Secondary serial port (PS1/PS2 adapter)",
  "field.VideoDevice": "Camera device",
  "field.ConsoleOutputVerbose0to2": "Log level",

  "help.Joystick": "Mouse and keyboard is Sinden's default; joystick reports X/Y axes and buttons 1 to 10.",
  "help.LeftHanded": "Left-handed swaps the left and right side buttons, in both modes.",
  "help.GangstaSetting": "The gun held on its side.",
  "help.ResourceMode": "Max processes every frame, medium one in two, low only while the trigger is held.",
  "help.Smooth": "Reduces the jitter of the aimed position.",
  "help.CameraExposure": "Empty: not set.",
  "help.CameraRes": "Width,height (e.g. 320,240).",
  "help.Calibrate": "Empty: read from the gun.",
  "help.ColourMatchRGB": "The colour of the screen border the camera looks for.",
  "help.MatchOnlyWherePointing": "Read but without effect in this driver.",
  "help.SerialPortWrite": "Empty: automatic. A number picks the n-th gun plugged.",

  "mode.mouse": "Mouse and keyboard",
  "mode.joystick": "Joystick",
  "hand.right": "Right-handed",
  "hand.left": "Left-handed",
  "grip.off": "Off",
  "grip.left": "Tilted left",
  "grip.right": "Tilted right",
  "resource.max": "Max",
  "resource.medium": "Medium",
  "resource.low": "Low",
  "recoil.single": "Single shot",
  "recoil.auto": "Automatic",
  "exposure.manual": "Manual",
  "exposure.auto": "Automatic",
  "log.info": "Information",
  "log.debug": "Debug",
  "log.trace": "Trace",

  "button.Trigger": "Trigger",
  "button.PumpAction": "Pump action",
  "button.FrontLeft": "Front left",
  "button.FrontRight": "Front right",
  "button.RearLeft": "Rear left",
  "button.RearRight": "Rear right",
  "button.Up": "D-pad up",
  "button.Down": "D-pad down",
  "button.Left": "D-pad left",
  "button.Right": "D-pad right",
  "buttons.button": "Button",
  "buttons.onScreen": "On screen",
  "buttons.offScreen": "Off screen",
  "buttons.modifier": "Modifier",
  "buttons.active": "active mode",
  "buttons.handedNote": "Left-handed swaps the left and right buttons when the map is sent to the gun.",

  "action.code": "Code {code}",
  "action.mouseLeft": "Mouse left button",
  "action.mouseMiddle": "Mouse middle button",
  "action.mouseRight": "Mouse right button",
  "action.key": "Key {arg}",
  "action.arrowUp": "Up arrow",
  "action.arrowDown": "Down arrow",
  "action.arrowLeft": "Left arrow",
  "action.arrowRight": "Right arrow",
  "action.joystickButton": "Joystick button {arg}",

  "value.default": "default: {value}",
  "value.empty": "(empty)",
};

export type MsgKey = keyof typeof en;
export type Lang = "en" | "fr";

const fr: Record<MsgKey, string> = {
  "app.title": "Sinden Driver",
  "app.language": "Langue",
  "lang.auto": "Système",
  "lang.en": "English",
  "lang.fr": "Français",

  "service.connected": "Pilote {version} connecté",
  "service.disconnected": "Pilote injoignable",
  "service.disconnectedHelp":
    "Le service sinden-driver ne tourne pas ou cet utilisateur ne peut pas joindre {endpoint}. Les réglages seront modifiables dès qu'il répondra ; nouvel essai…",
  "service.apiMismatch":
    "Cette interface parle l'API de contrôle {expected} ; le pilote parle {actual}. Certains réglages peuvent ne pas s'appliquer.",

  "guns.title": "Pistolets",
  "guns.none": "Aucun pistolet détecté",
  "gun.player": "Joueur {player}",
  "gun.firmware": "micrologiciel {firmware}",
  "state.starting": "démarrage",
  "state.aiming": "vise l'écran",
  "state.aimed_away": "hors écran",
  "state.calibrating": "en calibration",
  "state.stopped": "arrêté",
  "state.unknown": "inconnu",

  "tab.player": "Joueur {player}",
  "tab.general": "Général",
  "player2.inherit": "Les réglages du joueur 2 absents du fichier reprennent la valeur du joueur 1.",

  "actions.save": "Enregistrer",
  "actions.revert": "Annuler",
  "actions.pending": "{count} modification(s) non enregistrée(s)",
  "actions.saved": "Enregistré. Le pilote a redémarré les pistolets.",
  "actions.invalid": "Corrigez les champs signalés avant d'enregistrer.",
  "actions.error": "Le pilote a refusé la modification : {error}",

  "general.autostart": "Lancer l'application de la barre des tâches à l'ouverture de session",
  "general.configFile": "Fichier de configuration",
  "general.endpoint": "Point de contrôle",
  "general.about": "sinden-ui est un logiciel libre (MIT). Le binaire sinden-driver a sa propre licence.",

  "section.mode": "Mode et latéralité",
  "section.buttons": "Boutons",
  "section.buttonsMouse": "Boutons en mode souris et clavier",
  "section.buttonsJoystick": "Boutons en mode joystick",
  "section.recoil": "Recul",
  "section.camera": "Caméra",
  "section.aim": "Décalages et calibration",
  "section.vision": "Analyse d'image",
  "section.devices": "Périphériques",
  "section.logging": "Journal",

  "field.Joystick": "Mode",
  "field.LeftHanded": "Latéralité",
  "field.GangstaSetting": "Tenue sur le côté",
  "field.ResourceMode": "Mode de ressources",
  "field.Smooth": "Anti-tremblement (lissage)",
  "field.OffscreenReload": "Rechargement hors écran",
  "field.IAgreeRecoilTermsInLicense": "J'accepte les conditions de la licence sur le recul",
  "field.EnableRecoil": "Activer le recul au démarrage",
  "field.RecoilStrength": "Force du recul",
  "field.RecoilTrigger": "Recul sur la gâchette",
  "field.RecoilTriggerOffscreen": "Recul sur la gâchette hors écran",
  "field.RecoilPumpActionOnEvent": "Recul à la pompe vers l'avant",
  "field.RecoilPumpActionOffEvent": "Recul à la pompe vers l'arrière",
  "field.RecoilFrontLeft": "Recul sur le bouton avant gauche",
  "field.RecoilFrontRight": "Recul sur le bouton avant droit",
  "field.RecoilBackLeft": "Recul sur le bouton arrière gauche",
  "field.RecoilBackRight": "Recul sur le bouton arrière droit",
  "field.TriggerRecoilNormalOrRepeat": "Recul de la gâchette",
  "field.AutoRecoilStrength": "Force du recul automatique",
  "field.AutoRecoilStartDelay": "Délai avant le recul automatique",
  "field.AutoRecoilDelayBetweenPulses": "Délai entre les impulsions automatiques",
  "field.CameraExposureAuto": "Exposition",
  "field.CameraExposure": "Temps d'exposition",
  "field.CameraBrightness": "Luminosité",
  "field.CameraContrast": "Contraste",
  "field.CameraRes": "Résolution de la caméra",
  "field.OffsetX": "Décalage horizontal (%)",
  "field.OffsetY": "Décalage vertical (%)",
  "field.OffsetXRatio": "Facteur horizontal",
  "field.OffsetYRatio": "Facteur vertical",
  "field.CalibrateX": "Calibration X",
  "field.CalibrateY": "Calibration Y",
  "field.EnableCalibration": "Autoriser le mode calibration",
  "field.AutoSaveCalibrationInLightgun": "Enregistrer la calibration dans le pistolet",
  "field.ColourMatchRGB": "Couleur du cadre",
  "field.ColourMatchRange": "Tolérance de couleur",
  "field.MatchOnlyWherePointing": "Chercher seulement où l'on vise",
  "field.SerialPortWrite": "Port série",
  "field.SerialPortSecondary": "Port série secondaire (adaptateur PS1/PS2)",
  "field.VideoDevice": "Périphérique caméra",
  "field.ConsoleOutputVerbose0to2": "Niveau de journal",

  "help.Joystick":
    "Souris et clavier est le mode par défaut de Sinden ; joystick envoie des axes X/Y et les boutons 1 à 10.",
  "help.LeftHanded": "Gaucher échange les boutons latéraux gauches et droits, dans les deux modes.",
  "help.GangstaSetting": "Le pistolet tenu sur le côté.",
  "help.ResourceMode":
    "Max traite chaque image, moyen une sur deux, faible seulement quand la gâchette est enfoncée.",
  "help.Smooth": "Réduit le tremblement de la position visée.",
  "help.CameraExposure": "Vide : non réglé.",
  "help.CameraRes": "Largeur,hauteur (par ex. 320,240).",
  "help.Calibrate": "Vide : lue dans le pistolet.",
  "help.ColourMatchRGB": "La couleur du cadre de l'écran que la caméra cherche.",
  "help.MatchOnlyWherePointing": "Lu mais sans effet dans ce pilote.",
  "help.SerialPortWrite": "Vide : automatique. Un nombre choisit le n-ième pistolet branché.",

  "mode.mouse": "Souris et clavier",
  "mode.joystick": "Joystick",
  "hand.right": "Droitier",
  "hand.left": "Gaucher",
  "grip.off": "Non",
  "grip.left": "Incliné à gauche",
  "grip.right": "Incliné à droite",
  "resource.max": "Max",
  "resource.medium": "Moyen",
  "resource.low": "Faible",
  "recoil.single": "Coup par coup",
  "recoil.auto": "Automatique",
  "exposure.manual": "Manuelle",
  "exposure.auto": "Automatique",
  "log.info": "Information",
  "log.debug": "Débogage",
  "log.trace": "Trace",

  "button.Trigger": "Gâchette",
  "button.PumpAction": "Pompe",
  "button.FrontLeft": "Avant gauche",
  "button.FrontRight": "Avant droit",
  "button.RearLeft": "Arrière gauche",
  "button.RearRight": "Arrière droit",
  "button.Up": "Croix haut",
  "button.Down": "Croix bas",
  "button.Left": "Croix gauche",
  "button.Right": "Croix droite",
  "buttons.button": "Bouton",
  "buttons.onScreen": "À l'écran",
  "buttons.offScreen": "Hors écran",
  "buttons.modifier": "Modificateur",
  "buttons.active": "mode actif",
  "buttons.handedNote":
    "Gaucher échange les boutons gauches et droits quand la configuration est envoyée au pistolet.",

  "action.code": "Code {code}",
  "action.mouseLeft": "Bouton gauche de la souris",
  "action.mouseMiddle": "Bouton du milieu de la souris",
  "action.mouseRight": "Bouton droit de la souris",
  "action.key": "Touche {arg}",
  "action.arrowUp": "Flèche haut",
  "action.arrowDown": "Flèche bas",
  "action.arrowLeft": "Flèche gauche",
  "action.arrowRight": "Flèche droite",
  "action.joystickButton": "Bouton de joystick {arg}",

  "value.default": "défaut : {value}",
  "value.empty": "(vide)",
};

export const MESSAGES: Record<Lang, Record<MsgKey, string>> = { en, fr };

let current: Lang = "en";

/** Picks the language from a preference ("auto", "en", "fr") and the system's languages. */
export function resolveLang(preference: string | null, systemLanguages: readonly string[]): Lang {
  if (preference === "en" || preference === "fr") return preference;
  for (const tag of systemLanguages) {
    const primary = tag.toLowerCase().split(/[-_]/)[0];
    if (primary === "fr" || primary === "en") return primary;
  }
  return "en";
}

export function setLang(lang: Lang): void {
  current = lang;
}

export function lang(): Lang {
  return current;
}

/** The message `key` in the current language, `{name}` placeholders filled. */
export function t(key: MsgKey, params: Record<string, string | number> = {}): string {
  const template = MESSAGES[current][key] ?? MESSAGES.en[key] ?? key;
  return template.replace(/\{(\w+)\}/g, (whole, name: string) =>
    name in params ? String(params[name]) : whole,
  );
}
