use super::{AnimationEffect, AnimationSequence, AnimationTrigger, SlideAnimation};

impl AnimationSequence {
    /// Generate the `<p:timing>` XML block for the entire sequence.
    ///
    /// Returns an empty string when the sequence is empty.
    #[must_use]
    pub fn to_xml_string(&self) -> String {
        if self.animations.is_empty() {
            return String::new();
        }

        let mut id_counter: u32 = 1;

        let mut click_groups: Vec<String> = Vec::new();
        let mut current_group_items: Vec<String> = Vec::new();

        for anim in &self.animations {
            let anim_xml = build_anim_par(anim, &mut id_counter);

            match anim.trigger {
                AnimationTrigger::OnClick => {
                    if !current_group_items.is_empty() {
                        click_groups.push(wrap_click_group(&current_group_items, &mut id_counter));
                        current_group_items.clear();
                    }
                    current_group_items.push(anim_xml);
                }
                AnimationTrigger::WithPrevious | AnimationTrigger::AfterPrevious => {
                    current_group_items.push(anim_xml);
                }
            }
        }

        if !current_group_items.is_empty() {
            click_groups.push(wrap_click_group(&current_group_items, &mut id_counter));
        }

        let main_seq_children = click_groups.join("");

        let root_id = id_counter;
        id_counter += 1;
        let main_seq_id = id_counter;

        format!(
            "<p:timing>\
               <p:tnLst>\
                 <p:par>\
                   <p:cTn id=\"{root_id}\" dur=\"indefinite\" restart=\"never\" nodeType=\"tmRoot\">\
                     <p:childTnLst>\
                       <p:seq concurrent=\"1\" nextAc=\"seek\">\
                         <p:cTn id=\"{main_seq_id}\" dur=\"indefinite\" nodeType=\"mainSeq\">\
                           <p:childTnLst>\
                             {main_seq_children}\
                           </p:childTnLst>\
                         </p:cTn>\
                         <p:prevCondLst><p:cond evt=\"onPrev\" delay=\"0\"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond></p:prevCondLst>\
                         <p:nextCondLst><p:cond evt=\"onNext\" delay=\"0\"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond></p:nextCondLst>\
                       </p:seq>\
                     </p:childTnLst>\
                   </p:cTn>\
                 </p:par>\
               </p:tnLst>\
             </p:timing>"
        )
    }
}

/// Wrap one or more animation `<p:par>` elements into a click-group.
fn wrap_click_group(items: &[String], id_counter: &mut u32) -> String {
    let group_id = *id_counter;
    *id_counter += 1;

    let children = items.join("");
    format!(
        "<p:par>\
           <p:cTn id=\"{group_id}\" fill=\"hold\">\
             <p:stCondLst><p:cond delay=\"0\"/></p:stCondLst>\
             <p:childTnLst>\
               {children}\
             </p:childTnLst>\
           </p:cTn>\
         </p:par>"
    )
}

/// Build the inner `<p:par>` for a single animation.
fn build_anim_par(anim: &SlideAnimation, id_counter: &mut u32) -> String {
    let par_id = *id_counter;
    *id_counter += 1;

    let preset_class = anim.effect.preset_class();
    let preset_id = anim.effect.preset_id();
    let delay = anim.delay_ms;

    let node_type = match anim.trigger {
        AnimationTrigger::OnClick => "clickEffect",
        AnimationTrigger::WithPrevious => "withEffect",
        AnimationTrigger::AfterPrevious => "afterEffect",
    };

    let behaviour_xml = build_behaviour_xml(anim, id_counter);

    format!(
        "<p:par>\
           <p:cTn id=\"{par_id}\" presetID=\"{preset_id}\" presetClass=\"{preset_class}\" \
                   presetSubtype=\"0\" fill=\"hold\" nodeType=\"{node_type}\">\
             <p:stCondLst><p:cond delay=\"{delay}\"/></p:stCondLst>\
             <p:childTnLst>\
               {behaviour_xml}\
             </p:childTnLst>\
           </p:cTn>\
         </p:par>"
    )
}

/// Build the child behaviour element(s) for an animation.
fn build_behaviour_xml(anim: &SlideAnimation, id_counter: &mut u32) -> String {
    let set_id = *id_counter;
    *id_counter += 1;

    let spid = anim.target_shape_id;
    let dur = anim.duration_ms;

    match &anim.effect {
        AnimationEffect::Entrance(_) => {
            format!(
                "<p:set>\
                   <p:cBhvr>\
                     <p:cTn id=\"{set_id}\" dur=\"{dur}\" fill=\"hold\">\
                       <p:stCondLst><p:cond delay=\"0\"/></p:stCondLst>\
                     </p:cTn>\
                     <p:tgtEl><p:spTgt spid=\"{spid}\"/></p:tgtEl>\
                   </p:cBhvr>\
                   <p:to><p:strVal val=\"visible\"/></p:to>\
                 </p:set>"
            )
        }
        AnimationEffect::Exit(_) => {
            format!(
                "<p:set>\
                   <p:cBhvr>\
                     <p:cTn id=\"{set_id}\" dur=\"{dur}\" fill=\"hold\">\
                       <p:stCondLst><p:cond delay=\"0\"/></p:stCondLst>\
                     </p:cTn>\
                     <p:tgtEl><p:spTgt spid=\"{spid}\"/></p:tgtEl>\
                   </p:cBhvr>\
                   <p:to><p:strVal val=\"hidden\"/></p:to>\
                 </p:set>"
            )
        }
        AnimationEffect::Emphasis(_) => {
            format!(
                "<p:animEffect transition=\"in\" filter=\"fade\">\
                   <p:cBhvr>\
                     <p:cTn id=\"{set_id}\" dur=\"{dur}\" fill=\"hold\">\
                       <p:stCondLst><p:cond delay=\"0\"/></p:stCondLst>\
                     </p:cTn>\
                     <p:tgtEl><p:spTgt spid=\"{spid}\"/></p:tgtEl>\
                   </p:cBhvr>\
                 </p:animEffect>"
            )
        }
        AnimationEffect::MotionPath(path) => {
            format!(
                "<p:anim calcmode=\"lin\" valueType=\"num\">\
                   <p:cBhvr>\
                     <p:cTn id=\"{set_id}\" dur=\"{dur}\" fill=\"hold\">\
                       <p:stCondLst><p:cond delay=\"0\"/></p:stCondLst>\
                     </p:cTn>\
                     <p:tgtEl><p:spTgt spid=\"{spid}\"/></p:tgtEl>\
                     <p:attrNameLst><p:attrName>ppt_x</p:attrName></p:attrNameLst>\
                   </p:cBhvr>\
                   <p:tavLst>\
                     <p:tav tm=\"0\"><p:val><p:strVal val=\"{path}\"/></p:val></p:tav>\
                   </p:tavLst>\
                 </p:anim>"
            )
        }
    }
}
