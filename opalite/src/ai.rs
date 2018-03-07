use std::{ collections::VecDeque, sync::mpsc };
use specs::{ Entities, FetchMut, ReadStorage, System, VecStorage, WriteStorage };
use crate::{ MapMessage, MessageQueue, Position };

#[derive(Clone, Debug)]
pub enum AiGoal {
    Move { start: Position, target: Position, path: VecDeque<Position> },
}

#[derive(Component)]
#[component(VecStorage)]
pub struct AiComponent {
    current_goal: Option<AiGoal>,
    goals: VecDeque<AiGoal>,

    goal_completed: Box<Fn(&Option<AiGoal>) -> Vec<AiGoal> + Send + Sync>,
    goal_failed: Box<Fn(&Option<AiGoal>) -> Vec<AiGoal> + Send + Sync>,
}

impl AiComponent {
    pub fn new(completed: Box<Fn(&Option<AiGoal>) -> Vec<AiGoal> + Send + Sync>, failed: Box<Fn(&Option<AiGoal>) -> Vec<AiGoal> + Send + Sync>) -> Self {
        AiComponent {
            current_goal: None,
            goals: VecDeque::new(),
            goal_completed: completed,
            goal_failed: failed,
        }
    }
}

#[derive(Clone)]
pub enum AiMessages { }

pub struct AiSystem {
    messages: mpsc::Receiver<AiMessages>,
    sender: MessageQueue<AiMessages>,
}

impl AiSystem {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let sender = MessageQueue::new(sender);

        Self {
            messages: receiver,
            sender: sender,
        }
    }

    pub fn sender(&self) -> MessageQueue<AiMessages> {
        self.sender.clone()
    }
}

impl<'a> System<'a> for AiSystem {
    type SystemData =  (Entities<'a>,
                        WriteStorage<'a, AiComponent>,
                        ReadStorage<'a, Position>,
                        FetchMut<'a, MessageQueue<MapMessage>>);

    fn run(&mut self, (entities, mut ais, positions, mut map_messages): Self::SystemData) {
        use specs::Join;

        for (entity, ai, position) in (&*entities, &mut ais, &positions).join() {
            let mut current_goal = {
                match &ai.current_goal {
                    Some(goal) => Some(goal.clone()),
                    None => {
                        if let Some(goal) = ai.goals.pop_front() {
                            Some(goal)
                        } else {
                            None
                        }
                    }
                }
            };

            use self::AiGoal::*;
            match &mut current_goal {
                Some(Move { target, path, .. }) => {
                    if position == target {
                        for new_goal in (ai.goal_completed)(&current_goal) {
                            ai.goals.push_back(new_goal);
                        }

                        // current goal completed!
                        current_goal = None;
                    } else {
                        if path.is_empty() {
                            let mut current_step = *position;
                            while (current_step.x, current_step.y) != (target.x, target.y) {
                                let next_step = if current_step.x < target.x {
                                    Position::new(1, 0, 0)
                                } else if current_step.x > target.x {
                                    Position::new(-1, 0, 0)
                                } else if current_step.y < target.y {
                                    Position::new(0, 1, 0)
                                } else if current_step.y > target.y {
                                    Position::new(0, -1, 0)
                                } else {
                                    Position::new(0, 0, 0)
                                };
                                path.push_back(next_step);
                                current_step += next_step;
                            }
                        }

                        if let Some(next_position) = path.pop_front() {
                            map_messages.send(MapMessage::Move {
                                entity: entity,
                                position: next_position,
                                absolute: false,
                                reply: None,
                            });
                        }
                    }
                },
                None => {
                    for new_goal in (ai.goal_completed)(&current_goal) {
                        ai.goals.push_back(new_goal);
                    }
                },
            }

            ai.current_goal = current_goal;
        }
    }
}
