#[macro_use]
extern crate crossbeam_channel;

use std::thread;
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender};

/// The following tests replicate the channel examples from "Go
/// by Example" in order to determine how ergonomic crossbeam-channel
/// is in comparison to Go.
/// Examples that make use of time.Timer and time.Tick have been
/// skipped due to Rust's lack of equivalent functionality.
#[cfg(test)]
mod tests {
    use super::*;

    /// Equivalent to:
    /// ```go
    /// func main() {
    ///     messages := make(chan string)
    /// 
    ///     go func() { messages <- "ping" }()
    ///
    ///     msg := <-messages
    ///     fmt.Println(msg)
    /// }
    /// ```
    #[test]
    fn channels() {
        let (tx, rx) = bounded(0);

        thread::spawn(move || tx.send("ping").unwrap());

        let msg = rx.recv().unwrap();
        println!("{}", msg);
    }

    /// Equivalent to:
    /// ```go
    /// func main() {
    ///     messages := make(chan string, 2)
    ///
    ///     messages <- "buffered"
    ///     messages <- "channel"
    ///
    ///     fmt.Println(<-messages)
    ///     fmt.Println(<-messages)
    /// }
    /// ```
    #[test]
    fn channel_buffering() {
        let (tx, rx) = bounded(2);

        tx.send("buffered").unwrap();
        tx.send("channel").unwrap();

        println!("{}", rx.recv().unwrap());
        println!("{}", rx.recv().unwrap());
    }

    /// Equivalent to:
    /// ```go
    /// func worker(done chan bool) {
    ///     fmt.Print("working...")
    ///     time.Sleep(time.Second)
    ///     fmt.Println("done")
    ///
    ///     done <- true
    /// }
    ///
    /// func main() {
    ///     done := make(chan bool, 1)
    ///     go worker(done)
    ///
    ///     <-done
    /// }
    /// ```
    #[test]
    fn channel_synchronization() {
        fn worker(done: Sender<bool>) {
            println!("working...");
            thread::sleep(Duration::from_millis(1000));
            println!("done");

            done.send(true).unwrap();
        }

        let (tx, rx) = bounded(1);
        thread::spawn(move || worker(tx));

        rx.recv().unwrap();
    }

    /// Equivalent to:
    /// ```go
    /// func ping(pings chan<- string, msg string) {
    ///     pings <- msg
    /// }
    ///
    /// func pong(pings <-chan string, pongs chan<- string) {
    ///     msg := <-pings
    ///     pongs <- msg
    /// }
    ///
    /// func main() {
    ///     pings := make(chan string, 1)
    ///     pongs := make(chan string, 1)
    ///     ping(pings, "passed message")
    ///     pong(pings, pongs)
    ///     fmt.Println(<-pongs)
    /// }
    /// ```
    #[test]
    fn channel_directions() {
        fn ping(pings: Sender<&'static str>, msg: &'static str) {
            pings.send(msg).unwrap();
        }

        fn pong(pings: Receiver<&'static str>, pongs: Sender<&'static str>) {
            let msg = pings.recv().unwrap();
            pongs.send(msg).unwrap();
        }

        let (ping_tx, ping_rx) = bounded(1);
        let (pong_tx, pong_rx) = bounded(1);
        ping(ping_tx, "passed message");
        pong(ping_rx, pong_tx);
        println!("{}", pong_rx.recv().unwrap());
    }

    /// Equivalent to:
    /// ```go
    /// func main() {
    ///     c1 := make(chan string)
    ///     c2 := make(chan string)
    ///
    ///     go func() {
    ///         time.Sleep(time.Second * 1)
    ///         c1 <- "one"
    ///     }()
    ///     go func() {
    ///         time.Sleep(time.Second * 2)
    ///         c2 <- "two"
    ///     }()
    ///
    ///     for i := 0; i < 2; i++ {
    ///         select {
    ///         case msg1 := <-c1:
    ///             fmt.Println("received", msg1)
    ///         case msg2 := <-c2:
    ///             fmt.Println("received", msg2)
    ///         }
    ///     }
    /// }
    /// ```
    #[test]
    fn select() {
        let (c1_tx, c1_rx) = bounded(0);
        let (c2_tx, c2_rx) = bounded(0);

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(1000));
            c1_tx.send("one").unwrap();
        });
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(1000));
            c2_tx.send("two").unwrap();
        });

        for _ in 0..2 {
            select_loop! {
                recv(c1_rx, msg1) => println!("received {}", msg1),
                recv(c2_rx, msg2) => println!("received {}", msg2),
            }
        }
    }

    /// Equivalent to:
    /// ```go
    /// func main() {
    ///     c1 := make(chan string, 1)
    ///     go func() {
    ///         time.Sleep(time.Second * 2)
    ///         c1 <- "result 1"
    ///     }()
    ///
    ///     select {
    ///     case res := <-c1:
    ///         fmt.Println(res)
    ///     case <-time.After(time.Second * 1):
    ///         fmt.Println("timeout 1")
    ///     }
    ///
    ///     c2 := make(chan string, 1)
    ///     go func() {
    ///         time.Sleep(time.Second * 2)
    ///         c2 <- "result 2"
    ///     }()
    ///     select {
    ///     case res := <-c2:
    ///         fmt.Println(res)
    ///     case <-time.After(time.Second * 3):
    ///         fmt.Println("timeout 2")
    ///     }
    /// }
    /// ```
    #[test]
    fn timeouts() {
        let (c1_tx, c1_rx) = bounded(1);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(2000));
            c1_tx.send("result 1").unwrap();
        });

        select_loop! {
            recv(c1_rx, res) => println!("{}", res),
            timed_out(Duration::from_millis(1000)) => {
                println!("timeout 1");
            },
        }

        let (c2_tx, c2_rx) = bounded(1);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(2000));
            c2_tx.send("result 2").unwrap();
        });

        select_loop! {
            recv(c2_rx, res) => println!("{}", res),
            timed_out(Duration::from_millis(3000)) => {
                println!("timeout 2");
            },
        }
    }

    /// Equivalent to:
    /// ```go
    /// func main() {
    ///     messages := make(chan string)
    ///     signals := make(chan bool)
    ///
    ///     select {
    ///     case msg := <-messages:
    ///         fmt.Println("received message", msg)
    ///     default:
    ///         fmt.Println("no message received")
    ///     }
    ///
    ///     msg := "hi"
    ///     select {
    ///     case messages <- msg:
    ///         fmt.Println("sent message", msg)
    ///     default:
    ///         fmt.Println("no message sent")
    ///     }
    ///
    ///     select {
    ///     case msg := <-messages:
    ///         fmt.Println("received message", msg)
    ///     case sig := <-signals:
    ///         fmt.Println("received signal", sig)
    ///     default:
    ///         fmt.Println("no activity")
    ///     }
    /// }
    /// ```
    #[test]
    fn non_blocking_channel_operations() {
        let (msg_tx, msg_rx): (Sender<&str>, Receiver<&str>) = bounded(0);
        let (_sig_tx, sig_rx): (Sender<bool>, Receiver<bool>) = bounded(0);

        select_loop! {
            recv(msg_rx, msg) => println!("received message {}", msg),
            would_block() => println!("no message received"),
        }

        let msg = "hi";
        select_loop! {
            send(msg_tx, msg) => println!("sent message {}", msg),
            would_block() => println!("no message sent"),
        }

        select_loop! {
            recv(msg_rx, msg) => println!("received message {}", msg),
            recv(sig_rx, sig) => println!("received signal {}", sig),
            would_block() => println!("no activity"),
        }
    }

    /// Equivalent to:
    /// ```go
    /// func main() {
    ///     jobs := make(chan int, 5)
    ///     done := make(chan bool)
    ///
    ///     go func() {
    ///         for {
    ///             j, more := <-jobs
    ///             if more {
    ///                 fmt.Println("received job", j)
    ///             } else {
    ///                 fmt.Println("received all jobs")
    ///                 done <- true
    ///                 return
    ///             }
    ///         }
    ///     }()
    ///
    ///     for j := 1; j <= 3; j++ {
    ///         jobs <- j
    ///         fmt.Println("sent job", j)
    ///     }
    ///     close(jobs)
    ///     fmt.Println("sent all jobs")
    ///
    ///     <-done
    /// }
    /// ```
    #[test]
    fn closing_channels() {
        let (job_tx, job_rx) = bounded(5);
        let (done_tx, done_rx) = bounded(0);

        thread::spawn(move || {
            loop {
                select_loop! {
                    recv(job_rx, j) => println!("received job {}", j),
                    disconnected() => {
                        println!("received all jobs");
                        done_tx.send(true).unwrap();
                        return
                    },
                }
            }
        });

        for i in 0..3 {
            job_tx.send(i).unwrap();
            println!("sent job {}", i);
        }
        drop(job_tx);
        println!("sent all jobs");

        done_rx.recv().unwrap();
    }

    /// Equivalent to:
    /// ```go
    /// func main() {
    ///     queue := make(chan string, 2)
    ///     queue <- "one"
    ///     queue <- "two"
    ///     close(queue)
    ///
    ///     for elem := range queue {
    ///         fmt.Println(elem)
    ///     }
    /// }
    /// ```
    #[test]
    fn range_over_channels() {
        let (queue_tx, queue_rx) = bounded(2);
        queue_tx.send("one").unwrap();
        queue_tx.send("two").unwrap();
        drop(queue_tx);

        for elem in queue_rx.iter() {
            println!("{}", elem);
        }
    }

    // Skipping Timers and Tickers

    /// Equivalent to:
    /// ```go
    /// func worker(id int, jobs <-chan int, results chan<- int) {
    ///     for j := range jobs {
    ///         fmt.Println("worker", id, "started  job", j)
    ///         time.Sleep(time.Second)
    ///         fmt.Println("worker", id, "finished job", j)
    ///         results <- j * 2
    ///     }
    /// }
    ///
    /// func main() {
    ///     jobs := make(chan int, 100)
    ///     results := make(chan int, 100)
    ///
    ///     for w := 1; w <= 3; w++ {
    ///         go worker(w, jobs, results)
    ///     }
    ///
    ///     for j := 1; j <= 5; j++ {
    ///         jobs <- j
    ///     }
    ///     close(jobs)
    ///
    ///     for a := 1; a <= 5; a++ {
    ///         <-results
    ///     }
    /// }
    /// ```
    #[test]
    fn worker_pools() {
        fn worker(id: i32, jobs: Receiver<i32>, results: Sender<i32>) {
            for j in jobs.iter() {
                println!("worker {} started job {}", id, j);
                thread::sleep(Duration::from_millis(1000));
                println!("worker {} finished job {}", id, j);
                results.send(j * 2).unwrap();
            }
        }

        let (job_tx, job_rx) = bounded(100);
        let (res_tx, res_rx) = bounded(100);

        for w in 1..4 {
            let job_rx_clone = job_rx.clone();
            let res_tx_clone = res_tx.clone();
            thread::spawn(move || worker(w, job_rx_clone, res_tx_clone));
        }

        for j in 1..6 {
            job_tx.send(j).unwrap();
        }
        drop(job_tx);

        for _ in 1..6 {
            res_rx.recv().unwrap();
        }
    }

    // Skipping Rate Limiting
}
