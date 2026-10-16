//! as VIEWS tipadas do vocabulário `pp*`/`Effect`/`params_0..14` (§13.9).

use super::Element;

/// View tipada de um bloco `<presets>` — o vocabulário pp* do §13.9.
#[derive(Debug, Clone, Copy)]
pub struct PresetView<'a> {
    pub(super) el: &'a Element,
}

impl<'a> PresetView<'a> {
    /// O elemento por baixo da view (para attrs genéricos/edit).
    pub fn element(&self) -> &'a Element {
        self.el
    }

    /// `ppName` — nome do preset (forma crua/escapada).
    pub fn pp_name(&self) -> Option<&'a str> {
        self.el.attr("ppName")
    }
    /// `ppBank` — banco (u8 no fio; aqui cru).
    pub fn pp_bank(&self) -> Option<&'a str> {
        self.el.attr("ppBank")
    }
    /// `ppID` — id do preset (u16 BE no fio; cru aqui).
    pub fn pp_id(&self) -> Option<&'a str> {
        self.el.attr("ppID")
    }
    /// `ppType` — tipo/ícone (u16 BE no fio; cru aqui).
    pub fn pp_type(&self) -> Option<&'a str> {
        self.el.attr("ppType")
    }
    /// `ppTypeName` — rótulo do tipo (ex.: "Pop").
    pub fn pp_type_name(&self) -> Option<&'a str> {
        self.el.attr("ppTypeName")
    }
    /// `ppIRNum` — IR associado (se houver).
    pub fn pp_ir_num(&self) -> Option<&'a str> {
        self.el.attr("ppIRNum")
    }

    /// Os 9 `<Effect>` do preset, na ordem do arquivo (x = 0..8).
    pub fn effects(&self) -> impl Iterator<Item = EffectView<'a>> + 'a {
        self.el
            .children()
            .iter()
            .filter(|c| c.name == "Effect")
            .map(|el| EffectView { el })
    }

    /// O `<Effect>` do módulo da cadeia (PRE/DST/AMP/NR/CAB/EQ/MOD/DLY/RVB).
    pub fn effect(&self, module: &str) -> Option<EffectView<'a>> {
        self.effects().find(|e| e.module() == module)
    }
}

/// View tipada de um `<Effect>` — attrs fixos + `params_0..14` (§13.9).
#[derive(Debug, Clone, Copy)]
pub struct EffectView<'a> {
    pub(super) el: &'a Element,
}

impl<'a> EffectView<'a> {
    /// O elemento por baixo da view (para attrs genéricos/edit).
    pub fn element(&self) -> &'a Element {
        self.el
    }

    /// `effectModuleName` — slot da cadeia (PRE..RVB).
    pub fn module(&self) -> &'a str {
        self.el.attr("effectModuleName").unwrap_or("")
    }
    /// `effectName` — nome do algoritmo (pode ter espaços de padding).
    pub fn name(&self) -> Option<&'a str> {
        self.el.attr("effectName")
    }
    /// `effectState` — "1" ativo / "0" bypass.
    pub fn state(&self) -> Option<&'a str> {
        self.el.attr("effectState")
    }
    /// `effectCode` parseado — `(nibble << 24) | index` (u32 decimal).
    pub fn code(&self) -> Option<u32> {
        self.el.attr_u32("effectCode")
    }
    /// `params_N` cru (N = 0..14; §13.9: preservar params_N 0..14 — há
    /// controles ocultos fora do parameters.json, ex.: Mic do CAB).
    pub fn param(&self, n: u8) -> Option<&'a str> {
        if n > 14 {
            return None;
        }
        self.el.attr(&format!("params_{n}"))
    }
}
