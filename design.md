****# 计网实验2 设计

## 状态机设计

- stage0 基本状态, 可以响应文件发送请求, 命令行输入
- stage1 发送等待同步状态, 等待另一方确认握手
- stage2 发送状态, 发送文件并等待ack
- stage3 接收等待同步, 等待服务端确认握手
- stage4 接收状态, 接收文件并发送ack
- stage5 等待对方发送时间
- stage6 等待对方接收时间

### 关键结构

connection: 维护本链接的信息
+ host: 目标主机
+ src:  当前主机
+ stage: 当前状态机状态
+ tocnt: 超时次数

packet {
    code,
    content,
}

### 码分配
0-99: 发送方序列号
100-199: 接收方序列号, ack(code-100)的包
200: 握手1
201: 握手2
202: 握手3
203: 请求时间
204: 关闭连接
其余: 保留

### 状态机转移

stage0
- comm/quit: exit(0)
- comm/time: send(202) & stage5
- comm/send: send(200) & stage3
- net/200: send(201) & stage1
- net/203: send(time) & stage6
- other: drop

stage1
- net/201: stage3
- timeout: send(201)
- break: stage0
- other: drop

stage2
- list<ack>: ack(q)
- timeout: resend
- break: stage0

stage3
- net/201: send(202) & stage4
- timeout: send(200)
- break: stage0

stage4
- list<pkt>: save(pkt) & ack(pkt)
- timeout: nothing
- break: stage0

stage5
- net/time: stage0
- timeout: send(202)
- break: stage0

stage6
- timeout: send(202/time)
- break: stage0


### API

try_recv()

stage0(Receiver<String>, Connection, Config) {

}

stage1(Connection, Config) {

}

stage2(Connection, packets, Config) {

}

stage3(Connection, Config) {

}

stage4(Connection, buffer, Config) {

}

stage5(Connection, Config) {

}

stage6(Connection, Config) {
    
}