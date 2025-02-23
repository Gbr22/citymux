use std::{collections::HashMap, sync::Arc};
use tracing_subscriber::field::debug;
use which::which;
use tokio::sync::Mutex;

use crate::{canvas::TerminalInfo, draw::find_span_dimensions, exit::exit, process::handle_process, span::{get_root_dimensions, Node, NodeData, Span, SpanChild, SpanDirection}, tty::spawn_interactive_process, Process, StateContainer, Vector2};

pub async fn create_span(state_container: StateContainer) -> Result<usize, Box<dyn std::error::Error>> {
    let active_id = state_container.get_state().active_id.load(std::sync::atomic::Ordering::Relaxed);
    let new_id = state_container.get_state().span_id_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed)+1;
    state_container.get_state().active_id.store(new_id, std::sync::atomic::Ordering::Relaxed);
    let root_rect = get_root_dimensions(state_container.clone()).await;
    {
        let state = state_container.get_state();
        let mut root_guard = state.root_node.lock().await;
        let root = root_guard.as_mut();
        match root {
            None => {
                let root_node = Node::new(new_id, NodeData::Void);
                *root_guard = Some(root_node);
                tracing::debug!("Created root node: {:?}", root_guard);
                return Ok(new_id);
            },
            Some(root) => {
                match &root.data {
                    NodeData::Void => {
                        let container_id = state_container.get_state().span_id_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed)+1;
                        let mut new_root = Node::new(container_id, NodeData::Void);
                        let old_root_as_child = SpanChild::new(root.clone()).with_size(1.0);
                        let new_child = SpanChild::new(Node::new(new_id, NodeData::Void)).with_size(1.0);
                        
                        let is_horizonal_axis_larger = root_rect.size.x > root_rect.size.y;
                        let direction = if is_horizonal_axis_larger {
                            SpanDirection::Horizontal
                        } else {
                            SpanDirection::Vertical
                        };
                        let mut span = Span::new(direction);
                        span.children.push(old_root_as_child);
                        span.children.push(new_child);
                        let span = NodeData::Span(span);
                        new_root.data = span;
                        *root = new_root;
                        tracing::debug!("Replaced root node: {:?}", root_guard);
                        return Ok(new_id);
                    },
                    NodeData::Span(span) => {
                        let active_sizes = find_span_dimensions(&root, active_id, root_rect);
                        let Some(active_sizes) = active_sizes else {
                            return Err("Could not find active sizes".into());
                        };
                        let result = root.find_by_id(active_id);
                        let (_, path) = match result {
                            Some(tuple) => tuple,
                            None => {
                                return Err(format!("Could not find active node with id: {}", active_id).into());
                            }
                        };
                        let parent_id = path.last();
                        let Some(parent_id) = parent_id else {
                            return Err("Could not find parent node id".into());
                        };
                        let parent_id = parent_id.to_owned();
                        let parent_sizes = find_span_dimensions(&root, parent_id, root_rect);
                        let Some(parent_sizes) = parent_sizes else {
                            return Err("Could not find parent sizes".into());
                        };
                        let parent_clone = root.find_by_id(parent_id);
                        let (parent_clone, _) = match parent_clone {
                            Some(tuple) => tuple,
                            None => {
                                return Err("Could not find parent node".into());
                            }
                        };
                        let parent_clone = parent_clone.clone();
                        match parent_clone.data {
                            NodeData::Void => {
                                return Err(format!("Parent: {:?} is void",&parent_clone).into());
                            },
                            NodeData::Span(span) => {
                                match span.direction {
                                    SpanDirection::Horizontal => {
                                        let total = span.children.iter().fold(0.0, |acc, child| acc + child.size);
                                        let avg = total / span.children.len() as f64;
                                        let size_of_new_child = avg;
                                        let new_total = total + size_of_new_child;
                                        let new_ratio = size_of_new_child / new_total;
                                        let new_width = parent_sizes.size.x as f64 * new_ratio;
                                        if active_sizes.size.y as f64 > new_width {
                                            let mut new_span = Span::new(SpanDirection::Vertical);
                                            let container_id = state_container.get_state().span_id_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed)+1;
                                            let active_node = root.find_by_id(active_id);
                                            let Some(active_node) = active_node else {
                                                return Err("Could not find active node".into());
                                            };
                                            let active_node = active_node.0;
                                            new_span.children.push(SpanChild::new(active_node.clone()).with_size(1.0));
                                            new_span.children.push(SpanChild::new(Node::new(new_id, NodeData::Void)).with_size(1.0));
                                            *active_node = Node::new(container_id, NodeData::Span(new_span));
                                            return Ok(new_id);
                                        }
                                    },
                                    SpanDirection::Vertical => {
                                        let total = span.children.iter().fold(0.0, |acc, child| acc + child.size);
                                        let avg = total / span.children.len() as f64;
                                        let size_of_new_child = avg;
                                        let new_total = total + size_of_new_child;
                                        let new_ratio = size_of_new_child / new_total;
                                        let new_height = parent_sizes.size.y as f64 * new_ratio;
                                        if active_sizes.size.x as f64 > new_height {
                                            let mut new_span = Span::new(SpanDirection::Horizontal);
                                            let container_id = state_container.get_state().span_id_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed)+1;
                                            let active_node = root.find_by_id(active_id);
                                            let Some(active_node) = active_node else {
                                                return Err("Could not find active node".into());
                                            };
                                            let active_node = active_node.0;
                                            new_span.children.push(SpanChild::new(active_node.clone()).with_size(1.0));
                                            new_span.children.push(SpanChild::new(Node::new(new_id, NodeData::Void)).with_size(1.0));
                                            *active_node = Node::new(container_id, NodeData::Span(new_span));
                                            return Ok(new_id);
                                        }
                                    },
                                };

                                let parent = root.find_by_id(parent_id);
                                let (parent, _) = match parent {
                                    Some(tuple) => tuple,
                                    None => {
                                        return Err("Could not find parent node".into());
                                    }
                                };
                                let span = match &mut parent.data {
                                    NodeData::Span(span) => span,
                                    _ => {
                                        return Err("Parent is not a span".into());
                                    }
                                };

                                let total = span.children.iter().fold(0.0, |acc, child| acc + child.size);
                                let avg = total / span.children.len() as f64;
                                span.children.push(SpanChild::new(Node::new(new_id, NodeData::Void)).with_size(avg));
                                tracing::debug!("Added new node to parent: {:?}", &parent);
                                return Ok(new_id);
                            },
                        }
                    }
                }
            }
        }
    }
}

pub async fn create_process(state_container: StateContainer) -> Result<Arc<Mutex<Process>>, Box<dyn std::error::Error>> {
    let new_id = create_span(state_container.clone()).await?;
    let size = Vector2 {
        x: 1,
        y: 1
    };
    let program = "cmd";
    let program = which(program)?.to_string_lossy().to_string();
    let env = HashMap::new();
    
    let result = spawn_interactive_process(&program, env, size).await?;
    let process = Process {
        stdin: Arc::new(Mutex::new(result.stdin)),
        stdout: Arc::new(Mutex::new(result.stdout)),
        terminal_info: Arc::new(Mutex::new(TerminalInfo::new(size))),
        terminal: Arc::new(Mutex::new(result.terminal)),
        span_id: new_id,
    };

    let process = Arc::new(Mutex::new(process));
    let processes = state_container.get_state().processes.clone();
    {
        let mut processes = processes.lock().await;
        let future = {
            let process = process.clone();
            let state_container = state_container.clone();
            async move {
                let result = handle_process(state_container, process).await;
                if let Err(e) = result {
                    tracing::error!("Error: {:?}", e);
                }
            }
        };
        
        processes.push(process.clone());
        {
            let state = state_container.get_state();
            let locked = state.process_channel.lock().await;
            match locked.as_ref() {
                Some(sender) => {
                    sender.send(Box::pin(future)).await?;
                },
                None => {
                    return Err("No process channel".into());
                }
            }
        }
    }

    Ok(process)
}

pub fn remove_node(root: &mut Node, id: usize) -> Result<Option<usize>, Box<dyn std::error::Error>> {
    if root.id == id {
        root.data = NodeData::Void;
        return Ok(None);
    }

    let result = root.find_by_id(id);
    let (_, path) = match result {
        Some(tuple) => tuple,
        None => {
            return Err(format!("Could not find node with id: {}", id).into());
        }
    };
    let parent = path.last();
    let Some(parent) = parent else {
        return Err("Could not find parent node id".into());
    };
    let parent = parent.to_owned();
    let parent = root.find_by_id(parent);
    let (parent, _) = match parent {
        Some(tuple) => tuple,
        None => {
            return Err("Could not find parent node".into());
        }
    };
    match &mut parent.data {
        NodeData::Span(span) => {
            let mut index = None;
            for (i, child) in span.children.iter().enumerate() {
                if child.node.id == id {
                    index = Some(i);
                    break;
                }
            }
            match index {
                Some(index) => {
                    span.children.remove(index);
                    let last = span.children.last();
                    match last {
                        Some(last) => {
                            return Ok(Some(last.node.id));
                        },
                        None => {
                            let parent_id = parent.id;
                            return remove_node(root, parent_id);
                        },
                    }
                },
                None => {
                    return Err("Could not find child index".into());
                }
            };
        },
        _ => {},
    }
    
    Err("Could not remove node".into())
}

pub async fn kill_active_span(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
    tracing::debug!("Killing active span");
    let active_id = {
        state_container.get_state().active_id.load(std::sync::atomic::Ordering::Relaxed)
    };

    kill_span(state_container, active_id).await
}

pub async fn kill_span(state_container: StateContainer, span_id: usize) -> Result<(), Box<dyn std::error::Error>> {
    tracing::debug!("Killing span: {}", span_id);
    remove_node_from_state(state_container.clone(), span_id).await?;
    kill_process(state_container, span_id).await?;

    Ok(())
}

pub async fn kill_process(state_container: StateContainer, span_id: usize) -> anyhow::Result<()> {
    let processes = state_container.get_state().processes.clone();
    {
        let mut processes = processes.lock().await;
        let mut delete_index = None;
        let mut index: usize = 0;
        for process in &*processes {
            let process = process.lock().await;
            if process.span_id == span_id {
                delete_index = Some(index);
                let mut terminal = process.terminal.lock().await;
                terminal.release().await?;
                break;
            }
            index += 1;
        }
        if let Some(index) = delete_index {
            processes.remove(index);
        }
    }

    Ok(())
}

pub async fn remove_node_from_state(state_container: StateContainer, span_id: usize) -> Result<(), Box<dyn std::error::Error>> {
    {
        let state = state_container.get_state();
        let mut root_guard = state.root_node.lock().await;
        let root = root_guard.as_mut();
        match root {
            None => {
                return Ok(());
            },
            Some(root) => {
                let new_active = remove_node(root, span_id)?;
                match new_active {
                    Some(new_active) => {
                        state.active_id.store(new_active, std::sync::atomic::Ordering::Relaxed);
                    },
                    None => {
                        exit();
                    },
                }
            },
        };
    }

    Ok(())
}