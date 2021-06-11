use gooey::{
    core::{Context, StyledWidget, WidgetId},
    widgets::{
        button::{Button, ButtonCommand},
        component::{Behavior, Component, ComponentBuilder, ComponentTransmogrifier},
        container::Container,
    },
    App,
};
use pliantdb::client::Client;
use shared::{ExampleApi, Request, Response};

enum DatabaseCommand {
    Initialize(DatabaseContext),
    Increment,
}

struct DatabaseContext {
    widget_id: WidgetId,
    context: Context<Component<Counter>>,
}

fn main() {
    let counter = launch_database_worker();

    App::default()
        .with(ComponentTransmogrifier::<Counter>::default())
        .run(|storage| Component::<Counter>::new(counter, storage))
}

#[derive(Debug)]
struct Counter {
    command_sender: flume::Sender<DatabaseCommand>,
    count: Option<u32>,
}

impl Behavior for Counter {
    type Content = Container;
    type Event = CounterEvent;
    type Widgets = CounterWidgets;

    fn create_content(&mut self, builder: &mut ComponentBuilder<Self>) -> StyledWidget<Container> {
        Container::from_registration(builder.register_widget(
            CounterWidgets::Button,
            Button::new(
                "Click Me!",
                builder.map_event(|_| CounterEvent::ButtonClicked),
            ),
        ))
    }
    fn initialize(component: &mut Component<Self>, context: &Context<Component<Self>>) {
        let _ = component
            .behavior
            .command_sender
            .send(DatabaseCommand::Initialize(DatabaseContext {
                context: context.clone(),
                widget_id: component
                    .registered_widget(&CounterWidgets::Button)
                    .unwrap()
                    .id()
                    .clone(),
            }));
    }

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        _context: &Context<Component<Self>>,
    ) {
        let CounterEvent::ButtonClicked = event;

        let _ = component
            .behavior
            .command_sender
            .send(DatabaseCommand::Increment);
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
enum CounterWidgets {
    Button,
}

#[derive(Debug)]
enum CounterEvent {
    ButtonClicked,
}

fn launch_database_worker() -> Counter {
    let (command_sender, command_receiver) = flume::unbounded();

    App::spawn(process_database_commands(command_receiver));

    Counter {
        command_sender,
        count: None,
    }
}

async fn process_database_commands(receiver: flume::Receiver<DatabaseCommand>) {
    let client = Client::new("ws://127.0.0.1:8081".parse().unwrap(), None)
        .await
        .unwrap();
    let mut context = None;
    while let Ok(command) = receiver.recv_async().await {
        match command {
            DatabaseCommand::Initialize(new_context) => context = Some(new_context),
            DatabaseCommand::Increment => {
                increment_counter(&client, context.as_ref().expect("never initialized")).await;
            }
        }
    }
}

async fn increment_counter(client: &Client<ExampleApi>, context: &DatabaseContext) {
    match client.send_api_request(Request::IncrementCounter).await {
        Ok(response) => {
            let Response::CounterIncremented(count) = response;
            context.context.send_command_to::<Button>(
                &context.widget_id,
                ButtonCommand::SetLabel(count.to_string()),
            );
        }
        Err(err) => {
            log::error!("Error sending request: {:?}", err);
            eprintln!("Error sending request: {:?}", err);
        }
    }
}
