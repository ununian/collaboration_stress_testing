手动创建 N 篇文档，然后为每篇文档 N * 2 个线程 （N1，N2），然后 N1、N2 分别通过 WebSocket 连接服务器， 然后按照下列步骤进行

1. 生成一个随机ID RID，并发送给 N1、N2
2. N1、N2 设置内部计数器 C1 = 0，C2 = 0
3. N1 使用 RID 创建一个 Paragraph Block，向 Block 中写入文本 "ping_${ RID }_${ C1 } "，C1 ++
4. N2 检查到 RID 的 Block 中的文本变化，并且结尾为 "ping_${ RID }_${ C2 } " 时
  a. 为真，向 Block 结尾写入 "pong_${ RID }_${ C2 }"，C2++
  b. 为假，不做处理
5. N1 检查到 RID 的 Block 中的文本变化，并且结尾为 "ping_${ RID }_${ C1 } " 时
  a. 为真，向 Block 结尾写入 "ping_${ RID }_${ C1 }"，C1++
  b. 为假，不做处理
6. N1，N2 均设有超时时间，每次写入时，内部记录相关时间间隔
