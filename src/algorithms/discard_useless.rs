use std::collections::HashSet;

use crate::problem::Problem;



impl Problem {


    pub fn discard_useless_passive_labels(&mut self) {
        let labels_active = self.active.labels_appearing();
        let labels_passive = self.passive.labels_appearing();
        let to_keep = labels_active.intersection(&labels_passive).cloned().collect();
        let newp = self.harden(&to_keep);
        self.active = newp.active;
        self.passive = newp.passive;

        let to_keep = self.active.labels_appearing();

        self.mapping_label_text.retain(|(l,_)|{
            to_keep.contains(l)
        });
        if let Some(x) = self.mapping_label_oldlabels.as_mut() {
            x.retain(|(l,_)|{
                to_keep.contains(l)
            })
        }

        //to fix
        //pub trivial_sets: Option<Vec<Vec<usize>>>,
        //pub coloring_sets: Option<Vec<Vec<usize>>>,
        //pub diagram_indirect: Option<Vec<(usize, usize)>>,
        //pub diagram_direct: Option<(Vec<(usize, Vec<usize>)>, Vec<(usize, usize)>)>,
    }



    pub fn discard_useless_stuff(&mut self) {
        //maybe useless labels prevent to discard some useless lines, and one can actually see that some lines are useless only after removing useless labels?
        self.passive.discard_non_maximal_lines();
        self.remove_weak_active_lines();
    }

    pub fn remove_weak_active_lines(&mut self) {

        let reachable = self.diagram_indirect_to_reachability_adj();

        // part 1: make groups smaller if possible
        for line in self.active.lines.iter_mut() {
            for part in line.parts.iter_mut() {
                let group = part.group.as_set();

                let mut newgroup = vec![];
                'outer: for &label in &group {
                    for &other in &group {
                        if label != other && reachable[&label].contains(&other) {
                            continue 'outer;
                        }
                    }
                    newgroup.push(label);
                }
                newgroup.sort();
                part.group.0 = newgroup;
            }
        }


        // part 2: remove lines by inclusion
        self.active.discard_non_maximal_lines_with_custom_supersets(Some(|h1 : &HashSet<usize>, h2 : &HashSet<usize>|{
            // h2 is superset of h1 if all elements of h1 have a successor in h2
            h2.iter().all(|x|{
                h1.iter().any(|y|{
                    reachable[x].contains(y)
                })
            })
        }));

        // remove from passive side the labels that do not appear anymore on the active side
        self.discard_useless_passive_labels();
    }
}


#[cfg(test)]
mod tests {

    use crate::problem::Problem;

    #[test]
    fn useless1() {
        let mut p = Problem::from_string("A AB AB\n\nB AB").unwrap();
        p.compute_diagram();
        p.remove_weak_active_lines();
        assert_eq!(format!("{}", p), "A B^2\n\nAB B\n");

        let mut p = Problem::from_string("M M M\nP UP UP\n\nM UP\nU U").unwrap();
        p.compute_diagram();
        p.remove_weak_active_lines();
        assert_eq!(format!("{}", p), "M^3\nP U^2\n\nM PU\nMU U\n");
    }

    #[test]
    fn useless2() {
        let mut p = Problem::from_string("A A A\nA A B\n A B B\n\nB AB").unwrap();
        p.compute_diagram();
        p.remove_weak_active_lines();
        assert_eq!(format!("{}", p), "A B^2\n\nAB B\n");

        let mut p = Problem::from_string("M M M\nP U P\nP U U\nP P P\n\nM UP\nU U").unwrap();
        p.compute_diagram();
        p.remove_weak_active_lines();
        assert_eq!(format!("{}", p), "M^3\nP U^2\n\nM PU\nMU U\n");
    }
}
