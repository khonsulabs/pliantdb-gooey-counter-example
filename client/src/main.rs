use gooey::{
    core::{Context, StyledWidget, WidgetId},
    widgets::{
        button::{Button, ButtonCommand},
        component::{Behavior, Component, ComponentBuilder, ComponentTransmogrifier},
        container::Container,
    },
    App,
};
use pliantdb::{
    client::Client,
    core::pubsub::{PubSub, Subscriber},
};
use shared::{ExampleApi, Request, Response, COUNTER_CHANGED_TOPIC, DATABASE_NAME};

enum DatabaseCommand {
    Initialize(DatabaseContext),
    Increment,
}

#[derive(Clone)]
struct DatabaseContext {
    widget_id: WidgetId,
    context: Context<Component<Counter>>,
}

fn main() {
    let (command_sender, command_receiver) = flume::unbounded();

    App::spawn(process_database_commands(command_receiver));

    let counter = Counter {
        command_sender,
        count: None,
    };

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

async fn process_database_commands(receiver: flume::Receiver<DatabaseCommand>) {
    let client = Client::new("ws://127.0.0.1:8081".parse().unwrap())
        .await
        .unwrap();
    let mut context = None;
    while let Ok(command) = receiver.recv_async().await {
        match command {
            DatabaseCommand::Initialize(new_context) => {
                App::spawn(watch_for_changes(client.clone(), new_context.clone()));
                context = Some(new_context);
            }
            DatabaseCommand::Increment => {
                increment_counter(&client, context.as_ref().expect("never initialized")).await;
            }
        }
    }
}

async fn watch_for_changes(client: Client<ExampleApi>, context: DatabaseContext) {
    let database = client.database::<()>(DATABASE_NAME).await.unwrap();
    let subscriber = database.create_subscriber().await.unwrap();
    subscriber
        .subscribe_to(COUNTER_CHANGED_TOPIC)
        .await
        .unwrap();
    while let Ok(message) = subscriber.receiver().recv_async().await {
        let new_count = message.payload::<u64>().unwrap();
        context.context.send_command_to::<Button>(
            &context.widget_id,
            ButtonCommand::SetLabel(new_count.to_string()),
        );
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
