use crossbeam_channel::{unbounded, Sender};
use floem::{
    ext_event::register_ext_trigger,
    reactive::{create_rw_signal, ReadSignal, RwSignal, Scope, Trigger, WriteSignal},
};
use futures_signals::{
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt},
};

fn writer_to_triggered_sender<V>(writer: WriteSignal<V>) -> (Sender<V>, Trigger)
where
    V: 'static,
{
    let (sender, receiver) = unbounded();

    let scope = Scope::new();
    let trigger = scope.create_trigger();
    scope.create_effect(move |_| {
        trigger.track();
        while let Ok(value) = receiver.try_recv() {
            writer.try_set(value);
        }
    });

    (sender, trigger)
}

fn feed_signal<S>(signal: S, writer: WriteSignal<S::Item>)
where
    S: Signal + Send + 'static,
    S::Item: Send + 'static,
{
    let (sender, trigger) = writer_to_triggered_sender(writer);
    tokio::spawn(signal.for_each(move |value| {
        if sender.send(value).is_ok() {
            register_ext_trigger(trigger);
        }
        async {}
    }));
}

pub trait SignalFloemExt: Signal + Sized
where
    Self: Send + 'static,
    Self::Item: Send + 'static,
{
    fn to_floem(self, floem_signal: RwSignal<Self::Item>) -> ReadSignal<Self::Item> {
        self.write_floem(floem_signal.write_only());
        floem_signal.read_only()
    }

    fn to_floem_init(self, initial_value: Self::Item) -> ReadSignal<Self::Item> {
        let signal = create_rw_signal(initial_value);
        self.to_floem(signal)
    }

    fn write_floem(self, writer: WriteSignal<Self::Item>) {
        feed_signal(self, writer);
    }
}

impl<S> SignalFloemExt for S
where
    S: Signal + Sized + Send + 'static,
    S::Item: Send + 'static,
{
}

fn feed_signal_vec<S>(signal_vec: S, writer: WriteSignal<S::Item>)
where
    S: SignalVec + Send + 'static,
    S::Item: Send + 'static,
{
    let (sender, trigger) = writer_to_triggered_sender(writer);
    tokio::spawn(
        signal_vec
            .map(move |value| {
                if sender.send(value).is_ok() {
                    register_ext_trigger(trigger);
                }
            })
            .for_each(|_| async {}),
    );
}

pub trait SignalVecFloemExt: SignalVec + Sized
where
    Self: Send + 'static,
    Self::Item: Send + 'static,
{
    fn to_floem(self, floem_signal: RwSignal<Self::Item>) -> ReadSignal<Self::Item> {
        self.write_floem(floem_signal.write_only());
        floem_signal.read_only()
    }

    fn to_floem_init(self, initial_value: Self::Item) -> ReadSignal<Self::Item> {
        let signal = create_rw_signal(initial_value);
        self.to_floem(signal)
    }

    fn write_floem(self, writer: WriteSignal<Self::Item>) {
        feed_signal_vec(self, writer);
    }
}

impl<S> SignalVecFloemExt for S
where
    S: SignalVec + Sized + Send + 'static,
    S::Item: Send + 'static,
{
}
