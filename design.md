1. server

stage0
-> '--quit' => quit
-> '--time' => send_time()
-> '--testgbn' => stage = 1 & send_data(205)

stage1
-> '200' => stage = 3
-> 'timeout' => send_data(205)
-> 'mutli-timeout' => stage = 0

stage2
-> 'data' => send(data) & wait()
-> 'ack' => update_ack(ack)
-> 'timeout' => resend()
-> 'multi-timeout' => stage = 0

200 ok
201 respond
202 quit
203 time
204 refuse
205 start session

2. client

stage0
--> 'quit' => send(quit) & recv()
--> 'time' => send(time) & recv()
--> 'test-gbn' => stage = 1 & send(200)

stage3
--> '205' => stage = 2 & send(200)
--> 'timeout' => send(200)
--> 'multi-timeout' => stage = 0

stage
--> 'data' => recv(data) & ack
--> 'timeout' => count
--> 'multi-timeout' => stage = 0

server, any one 
--> 'wrong ack' ==> refuse()

pseudo-code

server
tick = 100ms
while true {
    match stage {
        0 => stage0;
        1 => stage1;
        ...
        n => stagen;
    }
}

stage0() {
    result = read_stdin()
    match result {
        "quit" => return;
    }
    size = try_rec(buffer)
    if size != 0
        match buffer.char[0] {
            200 => send_data(200) && stage = 1;
            201 => return;
            202 => send_data(time);
        }

    sleep(tick);
}

stage1() {
    size = try_rec(buffer)
    if size <= 0 {
        timeout_cnt += 1;
    } else {
        match code {
            200 -> send(200) & stage = 3;
            - -> refuse();
        }
    }
    tick();
}

stage2() {
    if can_send()
        send;

    size = try_rec(buffer):
        timeout -> tick
        mout -> stage = 0;
        ack -> update_ack();
}

timeout_cnt = 0

Enum recv_result {
    time_out,
    multi_out,
    code(u8),
}

send() {
    for item in Q
        if Q.Ready {
            send(Q)
        }
    while try_recv() {
        if packet {
            for item in Q {
                if Q.code == packet.code & Q.onfly {
                    Q.status = Arrived;
                }
            }
        }
    }
    while Q.first.Arrived 
        Q.pop_front()
    if Q.empty() {
        Ok(0)
    }
}

---

### 选作设计

1. server

stage0
-> '--quit' => quit
-> '--time' => send_time()
-> '--testgbn' => stage = 1 & send_data(205)

stage1
-> '200' => stage = 3
-> 'timeout' => send_data(205)
-> 'mutli-timeout' => stage = 0

stage2
-> 'data' => send(data) & wait()
-> 'ack' => update_ack(ack)
-> 'timeout' => resend()
-> 'multi-timeout' => stage = 0

200 start session
201 quit
202 time
204 refuse
205 accept

2. client

stage0
--> 'quit' => send(quit) & recv()
--> 'time' => send(time) & recv()
--> 'test-gbn' => stage = 1 & send(200)

stage3
--> '205' => stage = 2 & send(200)
--> 'timeout' => send(200)
--> 'multi-timeout' => stage = 0

stage
--> 'data' => recv(data) & ack
--> 'timeout' => count
--> 'multi-timeout' => stage = 0


----

### 选作 pseudo-code

