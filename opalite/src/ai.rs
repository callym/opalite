use std::collections::VecDeque;
use cgmath::Vector3;
use specs::{ Entities, Fetch, FetchMut, ReadStorage, System, VecStorage, WriteStorage };
use crate::{ Map, MapMessage, Position, RLock, MessageSender };

#[derive(Clone, Debug)]
pub enum AiGoalDo {
    Replace(Vec<AiGoal>),
    Queue(Vec<AiGoal>),
    Continue,
}

#[derive(Clone, Debug)]
pub enum AiGoal {
    Move { start: Vector3<i32>, target: Vector3<i32>, path: VecDeque<Vector3<i32>> },
}

#[derive(Component)]
#[component(VecStorage)]
pub struct AiComponent {
    current_goal: Option<AiGoal>,
    goals: VecDeque<AiGoal>,

    goal_do: Box<Fn(&Option<AiGoal>) -> AiGoalDo + Send + Sync>,
    goal_completed: Box<Fn(&Option<AiGoal>) -> Vec<AiGoal> + Send + Sync>,
    goal_failed: Box<Fn(&Option<AiGoal>) -> Vec<AiGoal> + Send + Sync>,
}

impl AiComponent {
    pub fn new(
        goal_do: Box<Fn(&Option<AiGoal>) -> AiGoalDo + Send + Sync>,
        goal_completed: Box<Fn(&Option<AiGoal>) -> Vec<AiGoal> + Send + Sync>,
        goal_failed: Box<Fn(&Option<AiGoal>) -> Vec<AiGoal> + Send + Sync>
    ) -> Self {
        AiComponent {
            current_goal: None,
            goals: VecDeque::new(),
            goal_do,
            goal_completed,
            goal_failed,
        }
    }
}

#[derive(Clone)]
pub enum AiMessages { }

pub struct AiSystem;

impl AiSystem {
    pub fn new() -> Self {
        AiSystem
    }
}

impl<'a> System<'a> for AiSystem {
    type SystemData =  (Entities<'a>,
                        WriteStorage<'a, AiComponent>,
                        ReadStorage<'a, Position>,
                        Fetch<'a, RLock<Map>>,
                        FetchMut<'a, MessageSender<MapMessage>>);

    fn run(&mut self, (entities, mut ais, positions, map, mut map_messages): Self::SystemData) {
        use specs::Join;

        let map = map.read().unwrap();

        let generate_new_goals = |ai: &mut AiComponent, goal: &Option<AiGoal>| {
            for new_goal in (ai.goal_completed)(&goal) {
                ai.goals.push_back(new_goal);
            }
            ai.goals.pop_front()
        };

        for (entity, ai, _) in (&*entities, &mut ais, &positions).join() {
            let position = match map.location(&entity) {
                Some(position) => position,
                None => continue,
            };

            let current_goal = match &ai.current_goal {
                Some(goal) => Some(goal.clone()),
                None => if let Some(goal) = ai.goals.pop_front() {
                    Some(goal)
                } else {
                    generate_new_goals(ai, &None)
                },
            };

            // check if the current goal is complete
            let mut current_goal = match &current_goal {
                Some(Move { target, .. }) => if position == target {
                    generate_new_goals(ai, &current_goal)
                } else { current_goal },
                None => None,
            };

            match (ai.goal_do)(&current_goal) {
                AiGoalDo::Replace(goals) => {
                    ai.goals.clear();
                    for goal in goals {
                        ai.goals.push_back(goal);
                    }
                },
                AiGoalDo::Queue(goals) => {
                    for goal in goals {
                        ai.goals.push_back(goal);
                    }
                },
                AiGoalDo::Continue => ()
            };

            use self::AiGoal::*;
            match &mut current_goal {
                Some(Move { target, path, .. }) => {
                    if path.is_empty() {
                        let mut current_step = *position;
                        while (current_step.x, current_step.z) != (target.x, target.z) {
                            let next_step = if current_step.x < target.x {
                                Vector3::new(1, 0, 0)
                            } else if current_step.x > target.x {
                                Vector3::new(-1, 0, 0)
                            } else if current_step.z < target.z {
                                Vector3::new(0, 0, 1)
                            } else if current_step.z > target.z {
                                Vector3::new(0, 0, -1)
                            } else {
                                Vector3::new(0, 0, 0)
                            };
                            path.push_back(next_step);
                            current_step += next_step;
                        }
                    }

                    if let Some(new_location) = path.pop_front() {
                        map_messages.send(MapMessage::Move {
                            entity: entity,
                            new_location,
                            absolute: false,
                            reply: None,
                        });
                    }
                },
                None => (),
            }

            ai.current_goal = current_goal;
        }
    }
}
