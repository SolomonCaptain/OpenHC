package main

import (
	pb "server/proto"
	"fmt"
	"io/ioutil"
	"log"
	"net"
	"path/filepath"

	"google.golang.org/grpc"
)

const (
	imageDir = "./images" // PNG 文件存放目录，文件名格式 frame_%04d.png
)

type server struct {
	pb.UnimplementedPNGServiceServer
}

// GetPNGStream 实现流式发送 PNG 文件
func (s *server) GetPNGStream(req *pb.PNGRequest, stream pb.PNGService_GetPNGStreamServer) error {
	log.Printf("收到请求: 起始=%d, 结束=%d", req.StartFrame, req.EndFrame)

	for frame := req.StartFrame; frame <= req.EndFrame; frame++ {
		// 构造文件名（示例：frame_0001.png）
		filename := fmt.Sprintf("frame_%04d.png", frame)
		filePath := filepath.Join(imageDir, filename)

		// 读取文件
		data, err := ioutil.ReadFile(filePath)
		if err != nil {
			// 如果文件不存在，可选择跳过或返回错误
			log.Printf("Warning: 读取失败 %s: %v", filePath, err)
			continue
		}

		// 构造响应块
		chunk := &pb.PNGChunk{
			Data:       data,
			FrameIndex: int32(frame),
		}

		// 发送到流
		if err := stream.Send(chunk); err != nil {
			log.Printf("为帧发送流失败 %d: %v", frame, err)
			return err
		}

		log.Printf("已发送帧 %d (%d 字节)", frame, len(data))
	}

	return nil
}

func main() {
	lis, err := net.Listen("tcp", ":50051")
	if err != nil {
		log.Fatalf("无法监听端口: %v", err)
	}

	s := grpc.NewServer()
	pb.RegisterPNGServiceServer(s, &server{})

	log.Println("gRPC服务器正在监听: 50051")
	if err := s.Serve(lis); err != nil {
		log.Fatalf("服务启动失败 %v", err)
	}
}