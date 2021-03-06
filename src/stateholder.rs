use notifications;

pub enum StateHolderCommand{
    GetState(Sender<StateHolderResponse>),
    Append(notifications::Notification),
    Clear,
}

pub enum StateHolderResponse{
    State(Vec<notifications::Notification>),
}

pub struct StateHolder{
    port: Receiver<StateHolderCommand>,
    state: Vec<notifications::Notification>,
}


impl StateHolder {
    pub fn new() -> (StateHolder, StateHolderInterface) {
        let (channel, port) = channel::<StateHolderCommand>();
        let state_holder = StateHolder{
            port: port,
            state: Vec::new(),
        };
        let state_holder_interface = StateHolderInterface::new(channel);
        return (state_holder, state_holder_interface);
    }

    pub fn start(&mut self) {
        loop {
            match self.port.recv() {
                GetState(channel) =>
                    channel.send(State(self.state.clone())),
                Append(notification) =>
                    self.state.push(notification),
                Clear =>
                    self.state = Vec::new(),
            }
        }
    }
}

pub struct StateHolderInterface {
    channel_to_stateholder: Sender<StateHolderCommand>,
    channel_to_me: Sender<StateHolderResponse>,
    port: Receiver<StateHolderResponse>,
}

impl Clone for StateHolderInterface {
    fn clone(&self) -> StateHolderInterface {
        StateHolderInterface::new(self.channel_to_stateholder.clone())
    }
}

impl StateHolderInterface {
    pub fn new(channel_to_stateholder: Sender<StateHolderCommand>) -> StateHolderInterface {
        let (channel_to_me, port) = channel::<StateHolderResponse>();
        StateHolderInterface{
            channel_to_stateholder: channel_to_stateholder,
            channel_to_me: channel_to_me,
            port: port,
        }
    }

    pub fn get_state(&self) -> Vec<notifications::Notification>{
        self.channel_to_stateholder.send(GetState(self.channel_to_me.clone()));
        match self.port.recv() {
            State(notifications) => notifications
        }
    }

    pub fn add_notification(&self, notification: notifications::Notification) {
        self.channel_to_stateholder.send(Append(notification));
    }

    pub fn clear(&self) {
        self.channel_to_stateholder.send(Clear);
    }

}
