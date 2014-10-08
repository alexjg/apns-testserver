use notifications;

pub enum StateHolderCommand{
    GetState(Sender<StateHolderResponse>),
    Append(notifications::Notification),
}

pub enum StateHolderResponse{
    State(Vec<notifications::Notification>),
}

pub struct StateHolder{
    port: Receiver<StateHolderCommand>,
    state: Vec<notifications::Notification>,
}


impl StateHolder {
    pub fn new(port: Receiver<StateHolderCommand>) -> StateHolder {
        StateHolder{
            port: port,
            state: Vec::new(),
        }
    }

    pub fn start(&mut self) {
        loop {
            match self.port.recv() {
                GetState(channel) =>
                    channel.send(State(self.state.clone())),
                Append(notification) =>
                    self.state.push(notification),
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
        info!("Notification added");
        self.channel_to_stateholder.send(Append(notification));
    }

}
