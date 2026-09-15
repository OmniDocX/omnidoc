use super::*;

#[test]
fn test_parse_empty_data() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram">
  <dgm:ptLst/>
  <dgm:cxnLst/>
</dgm:dataModel>"#;
    let nodes = parse_smartart_nodes(xml).expect("should parse empty data");
    assert!(nodes.is_empty());
}

#[test]
fn test_parse_single_node() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"
               xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <dgm:ptLst>
    <dgm:pt modelId="1">
      <dgm:prSet/>
      <dgm:spPr/>
      <dgm:t>
        <a:bodyPr/>
        <a:p>
          <a:r>
            <a:t>Hello World</a:t>
          </a:r>
        </a:p>
      </dgm:t>
    </dgm:pt>
  </dgm:ptLst>
  <dgm:cxnLst/>
</dgm:dataModel>"#;
    let nodes = parse_smartart_nodes(xml).expect("should parse single node");
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].text, "Hello World");
    assert!(nodes[0].children.is_empty());
}

#[test]
fn test_parse_parent_child_nodes() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"
               xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <dgm:ptLst>
    <dgm:pt modelId="0" type="doc">
      <dgm:t><a:bodyPr/><a:p><a:r><a:t></a:t></a:r></a:p></dgm:t>
    </dgm:pt>
    <dgm:pt modelId="1">
      <dgm:t><a:bodyPr/><a:p><a:r><a:t>Parent</a:t></a:r></a:p></dgm:t>
    </dgm:pt>
    <dgm:pt modelId="2">
      <dgm:t><a:bodyPr/><a:p><a:r><a:t>Child A</a:t></a:r></a:p></dgm:t>
    </dgm:pt>
    <dgm:pt modelId="3">
      <dgm:t><a:bodyPr/><a:p><a:r><a:t>Child B</a:t></a:r></a:p></dgm:t>
    </dgm:pt>
  </dgm:ptLst>
  <dgm:cxnLst>
    <dgm:cxn modelId="10" type="parOf" srcId="0" destId="1" sibTransId="100" parTransId="200"/>
    <dgm:cxn modelId="11" type="parOf" srcId="1" destId="2" sibTransId="101" parTransId="201"/>
    <dgm:cxn modelId="12" type="parOf" srcId="1" destId="3" sibTransId="102" parTransId="202"/>
  </dgm:cxnLst>
</dgm:dataModel>"#;
    let nodes = parse_smartart_nodes(xml).expect("should parse parent-child nodes");
    // Root should be the doc node (modelId=0), which has "Parent" as child
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].children.len(), 1);
    let parent = &nodes[0].children[0];
    assert_eq!(parent.text, "Parent");
    assert_eq!(parent.children.len(), 2);

    let child_texts: Vec<&str> = parent.children.iter().map(|c| c.text.as_str()).collect();
    assert!(child_texts.contains(&"Child A"));
    assert!(child_texts.contains(&"Child B"));
}

#[test]
fn test_skip_presentation_nodes() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"
               xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <dgm:ptLst>
    <dgm:pt modelId="1">
      <dgm:t><a:bodyPr/><a:p><a:r><a:t>Real Node</a:t></a:r></a:p></dgm:t>
    </dgm:pt>
    <dgm:pt modelId="2" type="pres">
      <dgm:t><a:bodyPr/><a:p><a:r><a:t>Presentation Node</a:t></a:r></a:p></dgm:t>
    </dgm:pt>
    <dgm:pt modelId="3" type="sibTrans">
      <dgm:t><a:bodyPr/><a:p><a:r><a:t>Transition</a:t></a:r></a:p></dgm:t>
    </dgm:pt>
  </dgm:ptLst>
  <dgm:cxnLst/>
</dgm:dataModel>"#;
    let nodes = parse_smartart_nodes(xml).expect("should skip pres nodes");
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].text, "Real Node");
}

#[test]
fn test_smartart_struct() {
    let sa = SmartArt {
        data_xml: b"<data/>".to_vec(),
        colors_xml: Some(b"<colors/>".to_vec()),
        style_xml: None,
        layout_xml: None,
        drawing_xml: None,
    };
    assert_eq!(sa.data_xml, b"<data/>");
    assert!(sa.colors_xml.is_some());
    assert!(sa.style_xml.is_none());
}

#[test]
fn test_smartart_node_equality() {
    let a = SmartArtNode {
        text: "Test".to_string(),
        children: vec![],
    };
    let b = SmartArtNode {
        text: "Test".to_string(),
        children: vec![],
    };
    assert_eq!(a, b);
}
