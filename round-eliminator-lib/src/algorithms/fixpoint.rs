use std::{collections::{HashSet, HashMap, BTreeSet}, fmt::Display};

use chashmap::CHashMap;
use itertools::Itertools;

use crate::{problem::{Problem, DiagramDirect}, constraint::Constraint, group::{Label, Group, GroupType}, line::Line, algorithms::diagram::compute_direct_diagram, part::Part};
use serde::{Deserialize, Serialize};
use super::{event::EventHandler, maximize::{Operation}, diagram::{diagram_indirect_to_reachability_adj, diagram_to_indirect}};


#[derive(Clone,Debug,Serialize,Deserialize,Eq,PartialEq)]
pub struct FixpointDiagram {
    additional_orig : Vec<Label>,
    orig_labels : Vec<Label>,
    rightclosed : Vec<Vec<Label>>,
    diagram : Vec<(Label,Label)>,
    mapping_oldlabel_text : Vec<(Label, String)>,
    mapping_rightclosed_newlabel : Vec<(Vec<Label>,Label)>,
    mapping_label_newlabel : Vec<(Label,Label)>,
    mapping_newlabel_text : Vec<(Label, String)>,
    text : String,
    diagram_direct : DiagramDirect
}

impl FixpointDiagram {
    fn new(p : &Problem) -> FixpointDiagram {
        let labels = p.labels();
        let diagram_indirect = p.diagram_indirect.as_ref().unwrap();
        let successors =  diagram_indirect_to_reachability_adj(&labels, diagram_indirect);

        let rcs = right_closed_subsets(&labels, &successors);

        let mut fd = FixpointDiagram {
            orig_labels : labels,
            rightclosed : rcs,
            diagram : vec![],
            additional_orig : vec![],
            mapping_newlabel_text : vec![],
            mapping_oldlabel_text : p.mapping_label_text.iter().cloned().collect(),
            mapping_label_newlabel : vec![],
            mapping_rightclosed_newlabel : vec![],
            text : "".into(),
            diagram_direct : (vec![],vec![])
        };
        fd.compute_mappings();
        fd.assign_text();
        fd.compute_diagram();
        fd.compute_text();

        fd
    }

    fn compute_mappings(&mut self){

        self.mapping_rightclosed_newlabel = self.rightclosed.iter().enumerate().map(|(i,r)|{
            (r.clone(),i as Label)
        }).collect();


        let mapping_label_rightclosed : HashMap<Label,Vec<Label>> = self.orig_labels.iter().map(|&l|{
            let mut containing : Vec<_> = self.rightclosed.iter().filter(|set|set.contains(&l)).collect();
            containing.sort_by_key(|x|x.len());
            (l,containing[0].clone())
        }).collect();

        let mapping_rightclosed_newlabel : HashMap<_,_> = self.mapping_rightclosed_newlabel.iter().cloned().collect();
        let mut mapping_label_newlabel = HashMap::new();
        for &l in &self.orig_labels {
            let newl = mapping_rightclosed_newlabel[&mapping_label_rightclosed[&l]];
            mapping_label_newlabel.insert(l, newl);
        }
        self.mapping_label_newlabel = mapping_label_newlabel.into_iter().collect();

    }

    fn compute_diagram(&mut self){
        let mut diagram = vec![];
        let mapping_rightclosed_newlabel : HashMap<_,_> = self.mapping_rightclosed_newlabel.iter().cloned().collect();

        let rcs = &self.rightclosed;
        
        for s1 in rcs {
            for s2 in rcs {
                let set_s1 : HashSet<Label> = HashSet::from_iter(s1.iter().cloned());
                let set_s2 : HashSet<Label> = HashSet::from_iter(s2.iter().cloned());
                let new_s1 = mapping_rightclosed_newlabel[s1];
                let new_s2 = mapping_rightclosed_newlabel[s2];
                if set_s1.is_superset(&set_s2) {
                    diagram.push((new_s1,new_s2));
                }
            }
        }

        let newlabels : Vec<Label> = mapping_rightclosed_newlabel.values().cloned().collect();
        let diagram = diagram_to_indirect(&newlabels,&diagram);
        self.diagram_direct = compute_direct_diagram(&newlabels, &diagram);
        self.diagram = diagram;
    }

    fn assign_text(&mut self) {
        let oldtext : HashMap<_,_> = self.mapping_oldlabel_text.iter().cloned()
            .chain(self.additional_orig.iter().enumerate().map(|(i,&l)|(l,format!("(dup{})",i))))
            .collect();
        let onechar = oldtext.iter().all(|(_,t)|t.len() == 1);
        self.mapping_newlabel_text = self.mapping_rightclosed_newlabel.iter().map(|(r,l)|{
            if onechar {
                let mut text = r.iter().map(|ol|&oldtext[ol]).sorted().join("");
                if text == "" {
                    text = "∅".into();
                }
                if text.chars().count() > 1 {
                    text = format!("({})",text);
                }
                (*l,text)
            } else {
                let mut text = format!("({})",
                    r.iter().map(|ol|
                        oldtext[ol].chars().filter(|&c|c!='('&&c!=')').collect::<String>()
                    ).sorted().join("_")
                );
                if text == "()" {
                    text = "∅".into();
                }
                (*l,text)
            }
        }).collect();
    }

    fn duplicate_labels(&mut self, groups : &Vec<Vec<Label>>){
        let newlabel_to_rightclosed : HashMap<_,_> = self.mapping_rightclosed_newlabel.iter().map(|(r,n)|(*n,r.clone())).collect();
        let mut sets : Vec<_> = self.mapping_rightclosed_newlabel.iter().map(|(r,_)|HashSet::from_iter(r.iter().cloned())).collect();
        let mut next_fresh = self.mapping_oldlabel_text.iter().map(|(o,t)|*o).max().unwrap() + 1;
        let mut additional_orig = vec![];

        for group in groups {
            let group : Vec<HashSet<Label>> = group.iter().map(|l|HashSet::from_iter(newlabel_to_rightclosed[l].iter().cloned())).collect();

            let mut labels_to_add = vec![];
            for set in sets.iter_mut() {
                if group.iter().any(|todup|{
                    let hset : HashSet<Label> = HashSet::from_iter(set.iter().cloned());
                    let hdup : HashSet<Label> = HashSet::from_iter(todup.iter().cloned());
                    let added : HashSet<Label> = HashSet::from_iter(additional_orig.iter().cloned());
                    let mut diff = hset.difference(&hdup);
                    hset.is_superset(&hdup) && diff.all(|d|added.contains(d))
                }) {
                    let mut set = set.clone();
                    set.insert(next_fresh);
                    labels_to_add.push(set);
                } else {
                    if group.iter().any(|set_to_dup|set_to_dup.is_subset(set)) {
                        set.insert(next_fresh);
                    }
                }
            }
            sets.extend(labels_to_add.into_iter());

            additional_orig.push(next_fresh);
            next_fresh += 1;
        }


        /* 
        let all_dup : HashSet<Label> = groups.iter().flat_map(|group|group.iter().cloned()).collect();
        for label in all_dup {
            let mut origs = vec![];
            for (i,group) in groups.iter().enumerate() {
                if group.contains(&label) {
                    origs.push(additional_orig[i]);
                }
            }
            for toadd in origs.iter().cloned().powerset() {
                let set = newlabel_to_rightclosed[&label].iter().cloned().chain(toadd.into_iter()).collect();
                sets.push(set);
            }
        }*/

        let mut result = HashSet::new();
        for set in sets {
            let mut set : Vec<_> = set.into_iter().collect();
            set.sort();
            result.insert(set);
        }

        self.rightclosed = result.into_iter().collect();
        self.additional_orig = additional_orig;
        self.compute_mappings();
        self.assign_text();
        self.compute_diagram();
        self.compute_text();
    }

    pub fn compute_text(&mut self) {
        let mapping_oldlabel_text : HashMap<_,_> = self.mapping_oldlabel_text.iter().cloned().collect();
        let mapping_label_newlabel = &self.mapping_label_newlabel;
        let mapping_newlabel_text = &self.mapping_newlabel_text;
        let diagram = &self.diagram;
        let newlabels : Vec<Label> = mapping_newlabel_text.iter().map(|&(l,_)|l).collect();
        let mapping_newlabel_text : HashMap<_,_> = mapping_newlabel_text.iter().cloned().collect();

        let diagram = diagram_to_indirect(&newlabels,&diagram);
        let (_,diagram) = compute_direct_diagram(&newlabels, &diagram);
        let mut hdiagram = HashMap::new();
        for &(a,b) in diagram.iter() {
            hdiagram.entry(a).or_insert(vec![]).push(b);
        }
        let diagram : Vec<_> = hdiagram.into_iter().sorted_by_key(|(l,_)|&mapping_newlabel_text[l]).collect();

        let diagram = diagram.iter().map(|(a,b)|{
            format!("{} -> {}",mapping_newlabel_text[a],b.iter().map(|b|&mapping_newlabel_text[b]).sorted().join(" "))
        }).join("\n");
        let mapping = mapping_label_newlabel.iter().map(|(l,n)|format!("{} = {}",mapping_oldlabel_text[l],mapping_newlabel_text[n].clone())).join("\n");
        self.text = format!("# mapping from original labels to diagram labels\n{}\n# diagram edges\n{}\n",mapping,diagram);
    }
}

type Tracking = (Line, Line, Line, Vec<Vec<usize>>, Vec<(usize, usize, Operation)>);

pub enum FixpointType{
    Basic,
    Dup(Vec<Vec<Label>>),
    Custom(String),
    Loop
}

impl Problem {

    pub fn compute_default_fixpoint_diagram(&mut self, labels : Option<Vec<Label>>, eh: &mut EventHandler) {
        if let Some(sublabels) = &labels {
            let mut subproblem = self.harden_keep(&sublabels.iter().cloned().collect(), false);
            subproblem.discard_useless_stuff(false, eh);
            self.fixpoint_diagram = Some((labels,FixpointDiagram::new(&subproblem)));
        } else {
            self.fixpoint_diagram = Some((None,FixpointDiagram::new(self)));
        }
    }


    pub fn fixpoint_generic(&self, sublabels : Option<Vec<Label>>, fptype : FixpointType, eh: &mut EventHandler ) -> Result<(Self,Vec<(Label,Label)>,Vec<(Label,Label)>), &'static str> {
        if let Some(sublabels) = sublabels {
            let mut subproblem = self.harden_keep(&sublabels.iter().cloned().collect(), false);
            subproblem.discard_useless_stuff(false, eh);
            subproblem.fixpoint_diagram = self.fixpoint_diagram.clone();
            let (fixpoint, diagram, mapping_label_newlabel) = subproblem.fixpoint_generic(None, fptype, eh).unwrap();
            let mut newlabel_to_label : HashMap<Label,Label> = mapping_label_newlabel.into_iter().filter(|(l,_)|sublabels.contains(l)).map(|(l,n)|(n,l)).collect();
            let orig_newlabels : HashSet<_> = newlabel_to_label.keys().cloned().collect();
            let mut next_fresh = *self.labels().iter().max().unwrap_or(&0) + 1;
            let active_fp = fixpoint.active.edited(|g|{
                Group(g.0.iter().map(|&l|{
                    if newlabel_to_label.contains_key(&l) {
                        newlabel_to_label[&l]
                    } else {
                        next_fresh += 1;
                        *newlabel_to_label.entry(l).or_insert(next_fresh)
                    }
                }).collect())
            });
            let passive_fp = fixpoint.passive.edited(|g|{
                Group(g.0.iter().map(|&l|{
                    if newlabel_to_label.contains_key(&l) {
                        newlabel_to_label[&l]
                    } else {
                        next_fresh += 1;
                        *newlabel_to_label.entry(l).or_insert(next_fresh)
                    }
                }).collect())
            });
            let mut active = self.active.clone();
            let mut passive = self.passive.clone();
            let mut mapping_label_text : HashMap<_,_> = self.mapping_label_text.iter().cloned().collect();
            for (l,t) in &fixpoint.mapping_label_text {
                if newlabel_to_label.contains_key(&l) && !mapping_label_text.contains_key(&newlabel_to_label[&l]) {
                    mapping_label_text.insert(newlabel_to_label[&l],t.clone());
                }
            }
            let mapping_label_text : Vec<_> = mapping_label_text.into_iter().unique().collect();
            for line in active_fp.lines {
                active.lines.push(line);
            }

            let map : HashMap<_,_> = mapping_label_text.iter().cloned().collect();
            passive.maximize(eh);
            let reachability = diagram_indirect_to_reachability_adj(&fixpoint.labels(),&diagram);
            let mut passive = passive.edited(|g|{
                let mut g = HashSet::from_iter(g.0.iter().cloned());
                let mut to_add = vec![];
                for (n,l) in &newlabel_to_label {
                    let real_successors : HashSet<_> = reachability[n].iter().filter(|succ|orig_newlabels.contains(succ)).map(|succ|newlabel_to_label[succ]).collect();
                    if real_successors.is_subset(&g) {
                        to_add.push(l);
                    }
                }
                g.extend(to_add.into_iter());
                Group(g.into_iter().sorted().collect())
            });
            for line in passive_fp.lines {
                passive.lines.push(line);
            }

            let mut p = Problem {
                active,
                passive,
                mapping_label_text,
                mapping_label_oldlabels: None,
                mapping_oldlabel_labels: None,
                mapping_oldlabel_text: None,
                trivial_sets: None,
                coloring_sets: None,
                diagram_indirect: None,
                diagram_indirect_old: None,
                diagram_direct: None,
                orientation_coloring_sets: None,
                orientation_trivial_sets: None,
                orientation_given: None,
                fixpoint_diagram : None
            };
            p.compute_diagram(eh);
            p.discard_useless_stuff(true, eh);
            let fixpoint_labels : HashSet<_> = newlabel_to_label.values().cloned().collect();
            let merge_groups = p.diagram_direct.as_ref().unwrap().0.clone();
            println!("{:?}",merge_groups);
            for (_, group) in merge_groups {
                let mut rename_to = group.iter().filter(|l|fixpoint_labels.contains(l));
                if let Some(dest) = rename_to.next() {
                    for &from in &group {
                        p = p.relax_merge(from, *dest);
                    }
                }
            }

            return Ok((p,diagram,self.labels().into_iter().map(|x|(x,x)).collect()));
        } else {
            match fptype {
                FixpointType::Basic => { self.fixpoint_dup(None, eh) },
                FixpointType::Dup(dups) => { self.fixpoint_dup(Some(dups),eh) }
                FixpointType::Loop => { self.fixpoint_loop(eh) },
                FixpointType::Custom(s) => { self.fixpoint_custom(s,eh) },

            }
        }
    }

    pub fn fixpoint(&self, eh: &mut EventHandler) -> Result<(Self,Vec<(Label,Label)>,Vec<(Label,Label)>), &'static str> {
        self.fixpoint_dup(None, eh)
    }

    pub fn fixpoint_dup(&self, dup : Option<Vec<Vec<Label>>>, eh: &mut EventHandler) -> Result<(Self,Vec<(Label,Label)>,Vec<(Label,Label)>), &'static str> {
        let mut fd = if let Some((_,fd)) = self.fixpoint_diagram.clone() {
            fd
        } else {
            FixpointDiagram::new(self)
        };
        if let Some(dup) = dup {
            fd.duplicate_labels(&dup);
        }
        let mapping_label_newlabel = fd.mapping_label_newlabel.clone();
        let mapping_newlabel_text = fd.mapping_newlabel_text.clone();
        let diagram = fd.diagram.clone();
        //println!("{:?}\n{:?}\n{:?}",mapping_label_newlabel,mapping_newlabel_text,diagram);

        Ok((self.fixpoint_onestep(&mapping_label_newlabel, &mapping_newlabel_text, &diagram, None, None, eh)?.0, diagram, mapping_label_newlabel))
    }


    pub fn fixpoint_custom(&self, text_diag : String, eh: &mut EventHandler) -> Result<(Self,Vec<(Label,Label)>,Vec<(Label,Label)>), &'static str> {
        let text_mapping = text_diag.lines().filter(|line|!line.starts_with("#") && line.contains("=")).join("\n");
        let text_diagram = text_diag.lines().filter(|line|!line.starts_with("#") && (line.contains("->") || line.contains("<-"))).join("\n");

        let mapping_newlabel_text : Vec<_> = text_diagram.split_whitespace().flat_map(|w|w.split("<-")).flat_map(|w|w.split("->")).filter(|&s|s != "->" && s != "<-" && s != "").unique().enumerate().map(|(l,s)|(l as Label,s.to_owned())).collect();
        let mapping_text_newlabel : HashMap<_,_> = mapping_newlabel_text.iter().cloned().map(|(a,b)|(b,a)).collect();
        let mapping_oldtext_newtext : HashMap<_,_> = text_mapping.lines().map(|line|{
            let mut line = line.split("=");
            let a = line.next().unwrap().trim();
            let b = line.next().unwrap().trim();
            (a.to_owned(),b.to_owned())
        }).collect();

        let mapping_label_newlabel : Vec<_> = self.mapping_label_text.iter().map(|(l,s)|{
            if mapping_oldtext_newtext.contains_key(s) {
                (*l,mapping_text_newlabel[&mapping_oldtext_newtext[s]])
            } else {
                (*l,mapping_text_newlabel[s])
            }
            
        }).collect();

        let diagram : Vec<_> = text_diagram.split("\n").flat_map(|line|{
            let mut v = vec![];
            if line.contains("->") {
                let mut line = line.split("->");
                let a = line.next().unwrap();
                let b = line.next().unwrap();
                for a in a.split_whitespace() {
                    for b in b.split_whitespace() {
                        v.push((mapping_text_newlabel[a],mapping_text_newlabel[b]));
                    }
                }
            } else if line.contains("<-") {
                let mut line = line.split("<-");
                let b = line.next().unwrap();
                let a = line.next().unwrap();
                for a in a.split_whitespace() {
                    for b in b.split_whitespace() {
                        v.push((mapping_text_newlabel[a],mapping_text_newlabel[b]));
                    }
                }
            } 
            v.into_iter()
        }).collect();
        Ok((self.fixpoint_onestep(&mapping_label_newlabel, &mapping_newlabel_text, &diagram, None, None, eh)?.0,diagram,mapping_label_newlabel))
    }

    pub fn fixpoint_onestep(&self, mapping_label_newlabel : &Vec<(Label, Label)>, mapping_newlabel_text : &Vec<(Label, String)>, diagram : &Vec<(Label,Label)>, tracking : Option<&CHashMap<Line,Tracking>>, tracking_passive : Option<&CHashMap<Line,Tracking>>, eh: &mut EventHandler) -> Result<(Self,Constraint), &'static str> {
        let active = self.active.all_choices(true);
        let passive = self.passive.all_choices(true);
        let active = Constraint{ lines: active, is_maximized: false, degree: self.active.degree  };
        let passive = Constraint{ lines: passive, is_maximized: false, degree: self.passive.degree  };
        let mapping_label_newlabel : HashMap<_,_> = mapping_label_newlabel.iter().cloned().collect();
        let active = active.edited(|g| Group(vec![mapping_label_newlabel[&g.0[0]]]));
        let passive = passive.edited(|g| Group(vec![mapping_label_newlabel[&g.0[0]]]));
        let newlabels : Vec<Label> = mapping_newlabel_text.iter().map(|&(l,_)|l).collect();
        let diagram_indirect = diagram_to_indirect(&newlabels,&diagram);
        let diagram_indirect_rev = diagram_indirect.iter().map(|&(a,b)|(b,a)).collect();
        let active = procedure(&active, &newlabels, &diagram_indirect, &mapping_newlabel_text, tracking, eh)?;
        let passive = procedure(&passive, &newlabels, &diagram_indirect_rev, &mapping_newlabel_text, tracking_passive, eh)?;
        let passive_successors = diagram_indirect_to_reachability_adj(&newlabels,&diagram_indirect);
        let passive_before_edit = passive.clone();
        let passive = passive.edited(|g| Group(passive_successors[&g.0[0]].iter().cloned().sorted().collect()));

        let mut p = Problem {
            active,
            passive,
            mapping_label_text: vec![],
            mapping_label_oldlabels: None,
            mapping_oldlabel_labels: None,
            mapping_oldlabel_text: None,
            trivial_sets: None,
            coloring_sets: None,
            diagram_indirect: None,
            diagram_indirect_old: None,
            diagram_direct: None,
            orientation_coloring_sets: None,
            orientation_trivial_sets: None,
            orientation_given: None,
            fixpoint_diagram : None
        };
        p.mapping_label_text = mapping_newlabel_text.clone();
        Ok((p,passive_before_edit))
    }


    pub fn fixpoint_loop(&self, eh: &mut EventHandler) -> Result<(Self,Vec<(Label,Label)>,Vec<(Label,Label)>), &'static str> {
        let fd = if let Some((_,fd)) = self.fixpoint_diagram.clone() {
            fd
        } else {
            FixpointDiagram::new(self)
        };        
        let orig_diagram = self.diagram_indirect.as_ref().unwrap();
        let mut diagram = fd.diagram;
        let mapping_label_newlabel = fd.mapping_label_newlabel;
        let mut mapping_newlabel_text = fd.mapping_newlabel_text;
        let mut mapping_label_newlabel : HashMap<_, _> = mapping_label_newlabel.iter().cloned().collect();

        let mut all_expressions : HashSet<TreeNode<Label>> = HashSet::new();
        let mut i=0;
        let p = loop {
            i += 1;
            let tracking = CHashMap::new();
            let tracking_passive = CHashMap::new();

            // run the fixpoint procedure, keep track of how each line has been obtained
            let (mut p,passive_before_edit) = self.fixpoint_onestep(&mapping_label_newlabel.iter().map(|(&a,&b)|(a,b)).collect(),&mapping_newlabel_text,&diagram,Some(&tracking),Some(&tracking_passive),eh)?;
            p.compute_triviality(eh);
            // if the problem is trivial, we need to repeat with a different diagram
            if !p.trivial_sets.as_ref().unwrap().is_empty() /*&& i <3*/ {

                // we extract all subexpressions for all lines obtained, both active and passive
                let mapping : HashMap<_,_> = mapping_newlabel_text.iter().cloned().collect();
                let mut expressions = HashSet::new();
                for (lines,tracking,flip) in [(p.active.lines,tracking,false),(passive_before_edit.lines,tracking_passive,true)] {
                    let mut exprs = HashSet::new();
                    for line in lines {
                        let len = if let Some(rg) = tracking.get(&line) {
                            let (_,_,before_norm,_,_) = &*rg;
                            before_norm.parts.len()
                        } else {
                            line.parts.len()
                        };
                        for i in 0..len {
                            let expr = expression_for_line_at(&line,i,false, &tracking,&mapping).reduce_rep();
                            expr.get_all_subexpressions(&mut exprs);
                        }
                    }
                    for expr in exprs {
                        expressions.insert(if flip { expr.flip() } else {expr});
                    }
                }

                // if something goes wrong, the original labels may not appear in the result, so we add them
                for (_,&x) in &mapping_label_newlabel {
                    expressions.insert(TreeNode::Terminal(x));
                }

                // the current expressions are added to the ones that we use in the next try
                let label_to_oldlabel : HashMap<_,_> = mapping_label_newlabel.iter().map(|(&l,&n)|(n,l)).collect();
                for e in &expressions {
                    all_expressions.insert(e.convert(&label_to_oldlabel));
                }

                eh.notify("fixpoint autofix", 0, all_expressions.len());
                (diagram,mapping_newlabel_text,mapping_label_newlabel) = diagram_for_expressions(&all_expressions, orig_diagram, &self.mapping_label_text, eh);
            } else {
                break p;
            }
        };

        Ok((p,diagram,mapping_label_newlabel.iter().map(|(&a,&b)|(a,b)).collect()))
    }


}

fn diagram_for_expressions(expressions : &HashSet<TreeNode<Label>>, orig_diagram : &Vec<(Label,Label)>, mapping_label_text : &Vec<(Label,String)>, eh: &mut EventHandler) -> (Vec<(Label,Label)>,Vec<(Label,String)>,HashMap<Label,Label>) {
    let mapping_label_text : HashMap<_,_> = mapping_label_text.iter().cloned().collect();
    let map_label_expr : HashMap<_,_> = expressions.iter().cloned().enumerate().map(|(a,b)|(a as Label,b)).collect();
    let map_expr_label : HashMap<_,_> = expressions.iter().cloned().enumerate().map(|(a,b)|(b,a as Label)).collect();

    // the first edges of the diagram are just given by the structure of the expressions
    let mut new_diagram = vec![];
    for (&l,e) in &map_label_expr {
        if let TreeNode::Expr(a,b,op) = e {
            if op == &Operation::Union {
                new_diagram.push((map_expr_label[a],l));
                new_diagram.push((map_expr_label[b],l));
            }
            if op == &Operation::Intersection {
                new_diagram.push((l,map_expr_label[a]));
                new_diagram.push((l,map_expr_label[b]));
            }
        }
    }

    // we now just compute some mappings
    let mut new_labels : Vec<Label> = map_label_expr.keys().cloned().collect();

    let mut new_mapping_label_newlabel : HashMap<_,_> = map_label_expr.iter().filter_map(|(&l,e)| match e {
        TreeNode::Terminal(x) => { Some((*x,l)) },
        TreeNode::Expr(_,_,_) => None
    }).collect();



    // we also add to the current diagram the edges of the original diagram
    for (x,y) in orig_diagram {
        new_diagram.push((new_mapping_label_newlabel[x],new_mapping_label_newlabel[y]));
    }

    new_diagram = diagram_to_indirect(&new_labels,&new_diagram);
    new_diagram.sort();

    // we fix the diagram: a node that is the union of (a,b) must point to all common successors of a and b 
    loop{
        let before_edit = new_diagram.clone();
        let diagram_rev : Vec<_> = new_diagram.iter().map(|&(a,b)|(b,a)).collect();
        let successors = diagram_indirect_to_reachability_adj(&new_labels,&new_diagram);
        let predecessors = diagram_indirect_to_reachability_adj(&new_labels,&diagram_rev);
        for (&l,e) in &map_label_expr {
            if let TreeNode::Expr(a,b,op) = e {
                let a = map_expr_label[a];
                let b = map_expr_label[b];
                if op == &Operation::Union {
                    let commons : Vec<_> = successors[&a].intersection(&successors[&b]).collect();
                    for &common in commons {
                        new_diagram.push((l,common));
                    }
                }
                if op == &Operation::Intersection {
                    let commons : Vec<_> = predecessors[&a].intersection(&predecessors[&b]).collect();
                    for &common in commons {
                        new_diagram.push((common,l));
                    }
                }
            }
        }
        new_diagram = diagram_to_indirect(&new_labels,&new_diagram);
        new_diagram.sort();
        if before_edit == new_diagram {
            break;
        }
    }

    // we add a source and a sink to make sure that every pair of labels has some common successor and predecessor
    let max = new_labels.iter().max().unwrap();
    let source = (max+1) as Label;
    let sink = (max+2) as Label;
    for &l in &new_labels {
        new_diagram.push((source,l));
        new_diagram.push((l,sink));
    }
    new_labels.push(source);
    new_labels.push(sink);

    // we merge equivalent labels
    let (merges,_) = compute_direct_diagram(&new_labels, &new_diagram);
    for (l,g) in merges {
        for l2 in g {
            if l2 != l {
                new_diagram.retain(|&(a,b)|a != l2 && b != l2);
                new_labels.retain(|&a|a != l2);
                for (k,v) in new_mapping_label_newlabel.iter().map(|(&k,&v)|(k,v)).collect::<Vec<_>>().into_iter() {
                    if v == l2 {
                        new_mapping_label_newlabel.insert(k,l);
                    }
                }
            }
        }
    }

    // the diagram may still not satisfy the requirements
    // we first compute some sets that would contain the same relations as the diagram that we just computed
    new_diagram = diagram_to_indirect(&new_labels,&new_diagram);
    let reachability : Vec<(Label,BTreeSet<Label>)> = diagram_indirect_to_reachability_adj(&new_labels,&new_diagram).into_iter().map(|(k,mut v)|{v.insert(k); (k,v.into_iter().collect())}).collect();
    let mut sets : HashMap<_,_> = reachability.iter().map(|(l,v)|(v.clone(),{
        if l != &source && l != &sink {
            map_label_expr[l].convert(&mapping_label_text).to_string()
        } else if l == &source {
            "(Source)".into()
        } else {
            "(Sink)".into()
        }
    })).collect();

    loop {
        // we now check if something is wrong, and in case we add new labels
        let mut to_add = HashMap::new();
        for l1 in sets.keys() {
            for l2 in sets.keys() {
                eh.notify("fixpoint autofix", 0, sets.len() + to_add.len());

                let intersection : BTreeSet<Label> = l1.intersection(l2).cloned().collect();
                let mut common : Vec<_> = sets.keys().filter(|l|l.is_subset(&intersection)).cloned().collect();
                for l in common.clone().into_iter() {
                    common.retain(|x| x == &l || !x.is_subset(&l));
                }
                if common.len() != 1 {
                    to_add.insert(intersection, format!("({}_INT_{})",sets[l1].replace("(","[").replace(")","]"),sets[l2].replace("(","[").replace(")","]")));
                }
                let union : BTreeSet<Label> = l1.union(l2).cloned().collect();
                let mut common : Vec<_> = sets.keys().filter(|l|l.is_superset(&union)).cloned().collect();
                for l in common.clone().into_iter() {
                    common.retain(|x| x == &l || !x.is_superset(&l));
                }
                if common.len() != 1 {
                    to_add.insert(union, format!("({}_UNI_{})",sets[l1].replace("(","[").replace(")","]"),sets[l2].replace("(","[").replace(")","]")));
                }
            }
        }
        if to_add.is_empty() {
            break;
        } else {
            sets.extend(to_add.into_iter());
        }
    }

    let mapping_set_newlabel : HashMap<_,_> = sets.keys().cloned().enumerate().map(|(l,s)|(s,l as Label)).collect();
    let mapping_newlabel_set : HashMap<_,_> = sets.keys().cloned().enumerate().map(|(l,s)|(l as Label,s)).collect();
    let new_labels : Vec<Label> = mapping_newlabel_set.keys().cloned().collect();

    let mut new_diagram = vec![];
    for &l1 in &new_labels {
        for &l2 in &new_labels {
            let s_l1 = &mapping_newlabel_set[&l1];
            let s_l2 = &mapping_newlabel_set[&l2];
            if s_l1 != s_l2 && s_l1.is_superset(s_l2) {
                new_diagram.push((l1,l2));
            }
        }
    }

    let mapping_oldoldlabel_oldlabel = new_mapping_label_newlabel;
    let mapping_oldlabel_set : HashMap<_,_> = reachability.iter().cloned().collect();
    let mapping_set_oldlabel : HashMap<_,_> = reachability.iter().cloned().map(|(k,v)|(v,k)).collect();
    let mapping_oldlabel_oldoldlabel : HashMap<Label,Label> = mapping_oldoldlabel_oldlabel.iter().map(|(&k,&v)|(v,k)).collect();
    let new_mapping_label_newlabel : HashMap<_,_> = mapping_oldoldlabel_oldlabel.into_iter().map(|(l,n)|{
        (l,mapping_set_newlabel[&mapping_oldlabel_set[&n]])
    }).collect();

    let old_labels : HashSet<_> = new_mapping_label_newlabel.values().cloned().collect();
    let new_mapping_newlabel_text = mapping_newlabel_set.iter().map(|(&l,s)|{
        if old_labels.contains(&l) {
            (l,mapping_label_text[&mapping_oldlabel_oldoldlabel[&mapping_set_oldlabel[s]]].to_owned())
        } else {
            (l,format!("({})",l))
        }
        //(l,sets[s].clone())
    }).collect();

    (new_diagram,new_mapping_newlabel_text,new_mapping_label_newlabel)
}

fn rcs_helper(labels : &[Label], right: &HashMap<Label,HashSet<Label>>, result: &mut Vec<HashSet<Label>>, added: HashSet<Label>) {
    for &x in labels {
        let mut toadd = right[&x].clone();
        toadd.insert(x);
        if !added.contains(&x) && (added.is_empty() || !toadd.is_superset(&added)) {
            let mut new = added.clone();
            new.extend(toadd.into_iter());
            result.push(new.clone());
            rcs_helper(&labels[1..], right, result, new);
        }
    }
}

pub fn right_closed_subsets(labels : &[Label], successors : &HashMap<Label, HashSet<Label>>) -> Vec<Vec<Label>> {
    let mut result = vec![HashSet::new()];
    rcs_helper(labels, successors, &mut result, HashSet::new());
    result.into_iter().map(|set|set.into_iter().sorted().collect::<Vec<Label>>()).unique().sorted().collect()
}


fn procedure(constraint : &Constraint, labels : &[Label], diagram_indirect : &Vec<(Label, Label)>, mapping : &Vec<(Label, String)>, tracking : Option<&CHashMap<Line,Tracking>>, eh: &mut EventHandler) -> Result<Constraint, &'static str> {
    let becomes_star = 100;


    let mapping : HashMap<_,_> = mapping.iter().cloned().collect();

    let successors = diagram_indirect_to_reachability_adj(&labels,&diagram_indirect);
    let predecessors = diagram_indirect_to_reachability_adj(&labels,&diagram_indirect.iter().cloned().map(|(a,b)|(b,a)).collect());

    let mut unions = HashMap::<(Label,Label),Label>::new();
    let mut intersections = HashMap::<(Label,Label),Label>::new();

    for &l1 in labels {
        for &l2 in labels {
            let mut common : HashSet<Label> = successors[&l1].intersection(&successors[&l2]).cloned().collect();
            for l in common.clone().into_iter() {
                for r in successors[&l].iter().filter(|&&x|x != l) {
                    common.remove(r);
                }
            }
            if common.len() != 1 {
                return Err("The diagram does not satisfy the requirements");
            }
            //assert!(common.len() == 1);
            unions.insert((l1,l2),common.into_iter().next().unwrap());

            let mut common : HashSet<Label> = predecessors[&l1].intersection(&predecessors[&l2]).cloned().collect();
            for l in common.clone().into_iter() {
                for r in predecessors[&l].iter().filter(|&&x|x != l) {
                    common.remove(r);
                }
            }
            if common.len() != 1 {
                return Err("The diagram does not satisfy the requirements");
            }
            //assert!(common.len() == 1);
            intersections.insert((l1,l2),common.into_iter().next().unwrap());
        }
    }

    let f_is_superset = |g1 : &Group,g2 : &Group|{
        successors[&g2[0]].contains(&g1[0])
    };

    let f_union = |g1 : &Group,g2 : &Group|{ 
        Group(vec![unions[&(g1[0],g2[0])]])
    };

    let f_intersection = |g1 : &Group,g2 : &Group|{ 
        Group(vec![intersections[&(g1[0],g2[0])]])
    };

    let mut newconstraint = constraint.clone();
    newconstraint.is_maximized = false;


    newconstraint.maximize_custom(eh,true,false,tracking,f_is_superset, f_union, f_intersection);
    /*println!("obtained constraint");
    for line in &newconstraint.lines {
        println!("{}",line.to_string(&mapping));
    }*/

    Ok(newconstraint)
}


#[derive(Ord,PartialOrd,Eq,PartialEq,Hash,Clone)]
enum TreeNode<T> where T : Ord + PartialOrd + Eq + PartialEq + std::hash::Hash + Clone{
    Terminal(T),
    Expr(Box<TreeNode<T>>,Box<TreeNode<T>>,Operation)
}


fn expression_for_line_at(line : &Line, pos : usize, norm_pos : bool, how : &CHashMap<Line, (Line, Line, Line, Vec<Vec<usize>>, Vec<(usize, usize, Operation)>)>, mapping : &HashMap<Label,String>) -> TreeNode<Label> {
    if let Some(rg) = how.get(line) {
        let (l1,l2, _, norm_map, parts) = &*rg;
        let (p1,p2,op) = parts[if norm_pos {norm_map[pos][0]} else {pos}];
        let part1 = expression_for_line_at(l1, p1, true, how, mapping);
        let part2 = expression_for_line_at(l2, p2, true, how, mapping);
        let mut v = vec![part1,part2];
        v.sort();
        let part2 = v.pop().unwrap();
        let part1 = v.pop().unwrap();
        TreeNode::Expr(Box::new(part1),Box::new(part2),op)
    } else {
        TreeNode::Terminal(line.parts[pos].group[0])
    }

}

impl<T> std::fmt::Display for TreeNode<T> where T : Ord + PartialOrd + Eq + PartialEq + std::hash::Hash + Clone + std::fmt::Display {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r = match self {
            TreeNode::Terminal(x) => { x.to_string() },
            TreeNode::Expr(a,b,op) => {
                let part1 = a.to_string();
                let part2 = b.to_string();
                let op = if *op == Operation::Union { "∩" } else { "∪" };
                format!("({}{}{})",part1.replace("(","[").replace(")","]") , op , part2.replace("(","[").replace(")","]"))
            }
        };
        write!(f, "{}",r)
    }
}

impl<T> TreeNode<T> where T : Ord + PartialOrd + Eq + PartialEq + std::hash::Hash + Clone + Display {

    fn convert<U>(&self, map : &HashMap<T,U>) -> TreeNode<U> where U : Ord + PartialOrd + Eq + PartialEq + std::hash::Hash + Clone {
        match self {
            TreeNode::Terminal(x) => { TreeNode::Terminal(map[&x].clone()) },
            TreeNode::Expr(a,b,op) => {
                let part1 = a.convert(map);
                let part2 = b.convert(map);
                TreeNode::Expr(Box::new(part1),Box::new(part2),*op)
            }
        }
    }

    fn reduce_rep(&self) -> Self {
        let mut t = self.clone();
        loop {
            let nt = t.reduce();
            if nt == t {
                break;
            }
            t = nt;
        }
        t
    }

    fn reduce(&self) -> Self {
        match self {
            TreeNode::Terminal(_) => { self.clone() },
            TreeNode::Expr(a,b,op) => {
                if a == b {
                    return a.reduce();
                }
                if let TreeNode::Expr(a1,a2,aop) = &**a {
                    if aop == op && (a1 == b || a2 == b) {
                        return a.reduce();
                    }
                }
                if let TreeNode::Expr(b1,b2,bop) = &**b {
                    if bop == op && (b1 == a || b2 == a) {
                        return b.reduce();
                    }
                }
                return TreeNode::Expr(Box::new(a.reduce()),Box::new(b.reduce()),*op);
            }
        }
    }

    fn get_all_subexpressions(&self, r : &mut HashSet<Self>) {
        if !r.contains(self) {
            r.insert(self.clone());
            match self {
                TreeNode::Terminal(_) => {},
                TreeNode::Expr(a,b,_) => {
                    a.get_all_subexpressions(r);
                    b.get_all_subexpressions(r);
                }
            }
        }
    }

    /*fn mirror_expr(&self, map : &HashMap<T,T>) -> Self {
        match self {
            TreeNode::Terminal(x) => { TreeNode::Terminal(map[&x].clone()) },
            TreeNode::Expr(a,b,op) => {
                let op = if *op == Operation::Intersection { Operation::Union } else { Operation::Intersection };
                return TreeNode::Expr(Box::new(a.mirror_expr(map)),Box::new(b.mirror_expr(map)),op);
            }
        }
    }*/
    fn flip(&self) -> Self {
        match self {
            TreeNode::Terminal(x) => { TreeNode::Terminal(x.clone()) },
            TreeNode::Expr(a,b,op) => {
                let op = if *op == Operation::Intersection { Operation::Union } else { Operation::Intersection };
                return TreeNode::Expr(Box::new(a.flip()),Box::new(b.flip()),op);
            }
        }
    }

    fn result(&self) -> BTreeSet<T> {
        match self {
            TreeNode::Terminal(x) => { BTreeSet::from([x.clone()]) },
            TreeNode::Expr(a,b,op) => {
                let a = a.result();
                let b = b.result();
                if *op == Operation::Intersection {
                    a.union(&b).cloned().collect()
                } else {
                    a.intersection(&b).cloned().collect()
                }
            }
        }
    }
}

/* 
fn add_diagram_edges(&mut self){
    for (l,e) in &self.map_label_expression {
        self.successors.entry(*l).or_default().insert(*l);
        self.predecessors.entry(*l).or_default().insert(*l);
        if let TreeNode::Expr(a,b,op) = e {
            if *op == Operation::Intersection {
                self.successors.entry(self.map_expression_label[a]).or_default().insert(*l);
                self.successors.entry(self.map_expression_label[b]).or_default().insert(*l);
            }
            if *op == Operation::Union {
                self.successors.entry(*l).or_default().insert(self.map_expression_label[a]);
                self.successors.entry(*l).or_default().insert(self.map_expression_label[b]);
            }
        }
        
    }
}*/