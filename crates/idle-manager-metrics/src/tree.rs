//! The pure process-tree walk both probes share. Neither system call that
//! fills the table lives here — `proc_pss.rs` parses `/proc/<pid>/status` on
//! Linux, `process_tree.rs` walks a `CreateToolhelp32Snapshot` on Windows —
//! only the breadth-first descent over whatever table a probe already read.

use std::collections::{HashMap, VecDeque};

/// One process table row a probe read: its command name and the parent it
/// hangs off. Neither field is OS-specific, so the same shape holds a
/// `/proc/<pid>/status` row and a `PROCESSENTRY32` row alike.
pub(crate) struct ProcessEntry {
    pub(crate) command: String,
    pub(crate) parent: u32,
}

/// The given pid and every descendant of it, breadth first, from a parent map.
pub(crate) fn descend_from(root: u32, table: &HashMap<u32, ProcessEntry>) -> Vec<u32> {
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for (&pid, entry) in table {
        children.entry(entry.parent).or_default().push(pid);
    }
    for kids in children.values_mut() {
        kids.sort_unstable();
    }

    let mut order = Vec::new();
    let mut queue = VecDeque::from([root]);
    while let Some(pid) = queue.pop_front() {
        order.push(pid);
        for &child in children.get(&pid).map(Vec::as_slice).unwrap_or_default() {
            queue.push_back(child);
        }
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(rows: &[(u32, &str, u32)]) -> HashMap<u32, ProcessEntry> {
        rows.iter()
            .map(|&(pid, command, parent)| {
                (
                    pid,
                    ProcessEntry {
                        command: command.to_owned(),
                        parent,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn descend_from_walks_breadth_first_and_includes_the_root() {
        let table = table(&[
            (1, "own", 0),
            (2, "child-a", 1),
            (3, "child-b", 1),
            (4, "grandchild", 2),
        ]);

        let order = descend_from(1, &table);

        assert_eq!(order, vec![1, 2, 3, 4]);
    }

    #[test]
    fn descend_from_skips_a_process_outside_the_roots_lineage() {
        let table = table(&[(1, "own", 0), (2, "child", 1), (99, "unrelated", 0)]);

        let order = descend_from(1, &table);

        assert_eq!(order, vec![1, 2]);
    }
}
