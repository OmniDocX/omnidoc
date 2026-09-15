use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use unippt_core::{Deck, SceneObject};

const MAX_FORMULA_CHARS: usize = 64 * 1024;

#[derive(Debug, Error)]
pub enum MathError {
    #[error("找不到公式转换脚本 tools/math/convert.py")]
    ScriptMissing,
    #[error("公式内容为空或超过 64 KiB")]
    InvalidInput,
    #[error("没有可用的 Python 公式环境；请设置 UNIPPT_PYTHON")]
    PythonUnavailable,
    #[error("公式转换失败: {0}")]
    Conversion(String),
    #[error("公式转换 I/O 失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("公式转换结果无效: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize)]
pub struct FormulaRequest {
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct FormulaResponse {
    pub value: String,
}

#[derive(Debug, Serialize)]
struct BatchInput<'a> {
    direction: &'a str,
    items: Vec<BatchItem<'a>>,
}

#[derive(Debug, Serialize)]
struct BatchItem<'a> {
    key: &'a str,
    value: &'a str,
}

#[derive(Debug, Deserialize)]
struct BatchOutput {
    values: HashMap<String, String>,
    errors: HashMap<String, String>,
}

pub fn convert_one(direction: &str, value: &str) -> Result<String, MathError> {
    if value.trim().is_empty() || value.len() > MAX_FORMULA_CHARS {
        return Err(MathError::InvalidInput);
    }
    let output = convert_batch(direction, &[("formula".to_string(), value.to_string())])?;
    output.values.get("formula").cloned().ok_or_else(|| {
        MathError::Conversion(
            output
                .errors
                .get("formula")
                .cloned()
                .unwrap_or_else(|| "转换器没有返回结果".into()),
        )
    })
}

/// Fill canonical LaTeX for OMML formulas found during PPTX import.
/// A missing Python environment degrades gracefully: the original OMML remains.
pub fn enrich_imported_formulas(deck: &mut Deck) {
    let mut inputs = Vec::new();
    for slide in &deck.slides {
        collect_omml(&slide.objects, &mut inputs);
    }
    if inputs.is_empty() {
        return;
    }
    let Ok(output) = convert_batch("omml-to-latex", &inputs) else {
        return;
    };
    for slide in &mut deck.slides {
        apply_latex(&mut slide.objects, &output.values);
    }
}

/// Ensure every editable formula has native OMML before PPTX export.
pub fn ensure_native_formulas(deck: &mut Deck) -> Result<(), MathError> {
    let mut inputs = Vec::new();
    for slide in &deck.slides {
        collect_latex_without_omml(&slide.objects, &mut inputs);
    }
    if inputs.is_empty() {
        return Ok(());
    }
    let output = convert_batch("latex-to-omml", &inputs)?;
    if let Some((key, message)) = output.errors.iter().next() {
        return Err(MathError::Conversion(format!("{key}: {message}")));
    }
    for slide in &mut deck.slides {
        apply_omml(&mut slide.objects, &output.values);
    }
    Ok(())
}

fn collect_omml(objects: &[SceneObject], inputs: &mut Vec<(String, String)>) {
    for object in objects {
        if let Some(formula) = &object.formula {
            if formula.latex.is_empty() {
                if let Some(omml) = &formula.omml {
                    inputs.push((object.id.clone(), omml.clone()));
                }
            }
        }
        collect_omml(&object.children, inputs);
    }
}

fn collect_latex_without_omml(objects: &[SceneObject], inputs: &mut Vec<(String, String)>) {
    for object in objects {
        if let Some(formula) = &object.formula {
            if formula.omml.as_deref().is_none_or(str::is_empty) && !formula.latex.is_empty() {
                inputs.push((object.id.clone(), formula.latex.clone()));
            }
        }
        collect_latex_without_omml(&object.children, inputs);
    }
}

fn apply_latex(objects: &mut [SceneObject], values: &HashMap<String, String>) {
    for object in objects {
        if let (Some(formula), Some(latex)) = (&mut object.formula, values.get(&object.id)) {
            formula.latex.clone_from(latex);
        }
        apply_latex(&mut object.children, values);
    }
}

fn apply_omml(objects: &mut [SceneObject], values: &HashMap<String, String>) {
    for object in objects {
        if let (Some(formula), Some(omml)) = (&mut object.formula, values.get(&object.id)) {
            formula.omml = Some(omml.clone());
        }
        apply_omml(&mut object.children, values);
    }
}

fn convert_batch(direction: &str, items: &[(String, String)]) -> Result<BatchOutput, MathError> {
    let script = find_script().ok_or(MathError::ScriptMissing)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let stem = format!("unippt-math-{}-{stamp}", std::process::id());
    let input_path = std::env::temp_dir().join(format!("{stem}.in.json"));
    let output_path = std::env::temp_dir().join(format!("{stem}.out.json"));
    let payload = BatchInput {
        direction,
        items: items
            .iter()
            .map(|(key, value)| BatchItem { key, value })
            .collect(),
    };
    std::fs::write(&input_path, serde_json::to_vec(&payload)?)?;

    let mut ran = false;
    let mut last_error = String::new();
    for python in python_candidates() {
        match Command::new(&python)
            .arg(&script)
            .arg(&input_path)
            .arg(&output_path)
            .output()
        {
            Ok(result) => {
                ran = true;
                if result.status.success() && output_path.is_file() {
                    let bytes = std::fs::read(&output_path)?;
                    let _ = std::fs::remove_file(&input_path);
                    let _ = std::fs::remove_file(&output_path);
                    return Ok(serde_json::from_slice(&bytes)?);
                }
                last_error = String::from_utf8_lossy(&result.stderr).trim().to_string();
            }
            Err(error) => last_error = error.to_string(),
        }
    }
    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
    if ran {
        Err(MathError::Conversion(last_error))
    } else {
        Err(MathError::PythonUnavailable)
    }
}

fn python_candidates() -> Vec<String> {
    let mut candidates = Vec::new();
    if let Ok(value) = std::env::var("UNIPPT_PYTHON") {
        if !value.trim().is_empty() {
            candidates.push(value);
        }
    }
    candidates.extend(["python3".into(), "python".into()]);
    candidates
}

fn find_script() -> Option<PathBuf> {
    crate::runtime_paths::file("tools/math/convert.py")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latex_omml_round_trip_uses_parent_pipeline() {
        let omml = convert_one("latex-to-omml", r"E=mc^2").unwrap();
        assert!(omml.contains("<m:oMath"));
        let latex = convert_one("omml-to-latex", &omml).unwrap();
        assert!(latex.contains("E=m"));
        assert!(latex.contains("{c}^{2}"));
    }
}
