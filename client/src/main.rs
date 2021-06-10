use gooey::{
    core::{Context, StyledWidget},
    widgets::{
        button::{Button, ButtonCommand},
        component::{Behavior, Component, ComponentBuilder, ComponentTransmogrifier},
        container::Container,
    },
    App,
};
use pliantdb::client::Client;
use shared::{ExampleApi, Request, Response};
use tokio::runtime::{Handle, Runtime};

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let counter = {
        let tokio = tokio::runtime::Runtime::new().unwrap();
        let handle = tokio.handle().clone();
        let (shutdown_sender, shutdown_receiver) = flume::bounded(1);
        std::thread::spawn(move || {
            tokio.block_on(shutdown_receiver.recv_async()).unwrap();
            println!("Exiting thread");
        });

        let client = handle
            .block_on(async { Client::new("ws://127.0.0.1:8081".parse().unwrap(), None).await })
            .unwrap();
        Counter {
            tokio: handle,
            shutdown: shutdown_sender,
            client,
            count: None,
        }
    };

    App::default()
        .with(ComponentTransmogrifier::<Counter>::default())
        .run(|storage| Component::<Counter>::new(counter, storage))
}

#[derive(Debug)]
struct Counter {
    tokio: Handle,
    shutdown: flume::Sender<()>,
    client: Client<ExampleApi>,
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

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &Context<Component<Self>>,
    ) {
        let CounterEvent::ButtonClicked = event;

        let client = component.behavior.client.clone();
        let button = component
            .registered_widget(&CounterWidgets::Button)
            .unwrap();
        let context = context.clone();
        component.behavior.tokio.spawn(async move {
            match client.send_api_request(Request::IncrementCounter).await {
                Ok(response) => {
                    let Response::CounterIncremented(count) = response;
                    if let Some(state) = context.widget_state(button.id().id) {
                        let channels = state.channels::<Button>().expect("incorrect widget type");
                        channels.post_command(ButtonCommand::SetLabel(count.to_string()));
                    }
                }
                Err(err) => {
                    log::error!("Error sending request: {:?}", err);
                    eprintln!("Error sending request: {:?}", err);
                }
            }
        });
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
