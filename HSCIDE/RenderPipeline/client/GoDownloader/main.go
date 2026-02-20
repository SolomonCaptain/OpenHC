package main

import (
	"context"
	"flag"
	"fmt"
	"io"
	"log"
	"os"
	"syscall"
	"time"

	pb "GoDownloader/proto"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

const (
	numSlots     = 4               // 环形缓冲区槽位数
	slotDataSize = 1024 * 1024 * 4 // 4MB 每个槽位
	totalShmSize = ctrlSize + numSlots*(SlotHeaderSize+slotDataSize)
)

func main() {
	// 命令行参数
	var (
		shmName    string
		eventReady string
		eventFree  string
		cloudAddr  string
		startFrame int
		endFrame   int
	)
	flag.StringVar(&shmName, "shm", "", "共享内存名称 (例如 Global\\MyTask)")
	flag.StringVar(&eventReady, "event-ready", "", "数据就绪事件名称")
	flag.StringVar(&eventFree, "event-free", "", "槽位空闲事件名称")
	flag.StringVar(&cloudAddr, "cloud", "localhost:50051", "云端 gRPC 地址")
	flag.IntVar(&startFrame, "start", 1, "起始帧号")
	flag.IntVar(&endFrame, "end", 100, "结束帧号")
	flag.Parse()

	if shmName == "" || eventReady == "" || eventFree == "" {
		log.Fatal("必须指定 --shm, --event-ready, --event-free")
	}

	// 1. 打开/创建共享内存
	shm, err := CreateOrOpenSharedMemory(shmName, totalShmSize)
	if err != nil {
		log.Fatalf("共享内存创建失败: %v", err)
	}
	defer shm.Close()
	log.Printf("共享内存 %s 已映射，大小 %d 字节", shmName, totalShmSize)

	// 2. 打开/创建事件
	readyEvent, err := CreateEvent(eventReady, false) // auto-reset
	if err != nil {
		log.Fatalf("创建事件 %s 失败: %v", eventReady, err)
	}
	defer syscall.CloseHandle(readyEvent)

	freeEvent, err := CreateEvent(eventFree, false)
	if err != nil {
		log.Fatalf("创建事件 %s 失败: %v", eventFree, err)
	}
	defer syscall.CloseHandle(freeEvent)

	// 3. 初始化环形缓冲区
	rb := NewRingBuffer(shm.Data(), numSlots, slotDataSize)

	// 4. 连接云端 gRPC
	conn, err := grpc.Dial(cloudAddr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		log.Fatalf("连接云端失败: %v", err)
	}
	defer conn.Close()
	client := pb.NewPNGServiceClient(conn)

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Minute)
	defer cancel()

	stream, err := client.GetPNGStream(ctx, &pb.PNGRequest{
		StartFrame: int32(startFrame),
		EndFrame:   int32(endFrame),
	})
	if err != nil {
		log.Fatalf("调用云端流失败: %v", err)
	}

	log.Printf("开始接收帧 %d 到 %d", startFrame, endFrame)

	frameCount := 0
	expectedFrames := endFrame - startFrame + 1

	// 5. 主循环：接收流并写入共享内存
	for {
		// 等待空闲槽位事件（表示有槽位可用）
		// 如果缓冲区初始有空闲槽，不会阻塞；但为了简单，我们循环内尝试获取，若无空闲则等待事件。
		// 实际流程：先尝试获取写槽，若失败则等待 freeEvent，超时则报错。
		var slotIdx int
		var slotBuf []byte
		for {
			idx, buf, err := rb.GetWriteSlot()
			if err == nil {
				slotIdx = idx
				slotBuf = buf
				break
			}
			// 缓冲区满，等待空闲事件
			log.Println("缓冲区满，等待空闲事件...")
			_, err = WaitForSingleObject(freeEvent, syscall.INFINITE)
			if err != nil {
				log.Fatalf("等待空闲事件失败: %v", err)
			}
			// 再次尝试
		}

		// 接收下一个数据块
		chunk, err := stream.Recv()
		if err == io.EOF {
			// 所有帧接收完毕，写入终止帧
			log.Println("云端流结束，发送终止标记")
			rb.CommitWrite(slotIdx, 0, true)
			// 触发 ready 事件，通知渲染器有终止帧
			if err := SetEvent(readyEvent); err != nil {
				log.Printf("设置就绪事件失败: %v", err)
			}
			break
		}
		if err != nil {
			log.Fatalf("接收流错误: %v", err)
		}

		// 检查数据大小是否超过槽位容量
		if len(chunk.Data) > slotDataSize {
			log.Fatalf("PNG 数据过大 (%d 字节)，槽位容量 %d", len(chunk.Data), slotDataSize)
		}

		// 复制数据到共享内存槽
		copy(slotBuf, chunk.Data)

		// 提交写入
		rb.CommitWrite(slotIdx, len(chunk.Data), false)

		// 触发 ready 事件，通知渲染器有新数据
		if err := SetEvent(readyEvent); err != nil {
			log.Printf("设置就绪事件失败: %v", err)
		}

		frameCount++
		log.Printf("帧 %d 已写入共享内存 (索引 %d)", chunk.FrameIndex, slotIdx)

		// 检查帧号连续性
		if int(chunk.FrameIndex) != startFrame+frameCount-1 {
			log.Printf("警告：帧号不连续，期望 %d 实际 %d", startFrame+frameCount-1, chunk.FrameIndex)
		}
	}

	log.Printf("所有 %d 帧已写入，等待渲染器处理完成", frameCount)

	time.Sleep(5 * time.Second)
	log.Println("下载器退出")
}
