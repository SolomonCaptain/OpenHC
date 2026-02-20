package main

import (
	"encoding/binary"
	"errors"
)

// 环形缓冲区布局（位于共享内存起始处）
// 头部：槽位数，槽位大小，写索引，读索引，终止标志，事件句柄存储（实际上事件句柄由外部维护）
// 为了简化，我们将控制结构定义为一个 Go struct，映射到共享内存头部。

// Slot 代表一个数据槽（位于共享内存头部之后）
// 每个槽位固定大小 MAX_SLOT_SIZE，前4字节为状态（0=空闲，1=就绪，2=终止），随后4字节为数据长度，后面是数据。
const (
	SlotStatusFree  = 0
	SlotStatusReady = 1
	SlotStatusTerm  = 2 // 终止帧（表示结束）
	SlotHeaderSize  = 8 // 状态（4）+ 长度（4）
)

// RingBuffer 封装了共享内存中的环形缓冲区操作
type RingBuffer struct {
	mem           []byte
	numSlots      int
	slotDataSize  int // 每个槽位实际可存储的数据大小（不包含头部）
	totalSlotSize int // 头部+数据
	headerOffset  int // 头部大小（存储槽位数、索引等）
	// 控制字段偏移（位于共享内存起始）
	// 布局：写索引(4) + 读索引(4) + 终止标志(1) + 保留(3) + [可选numSlots, slotSize固定]
}

const (
	offWriteIdx = 0
	offReadIdx  = 4
	offTermFlag = 8
	ctrlSize    = 12
)

// NewRingBuffer 基于共享内存初始化环形缓冲区
// 如果内存未初始化（全零），则初始化控制字段；否则使用已有控制字段。
func NewRingBuffer(shm []byte, numSlots, slotDataSize int) *RingBuffer {
	rb := &RingBuffer{
		mem:           shm,
		numSlots:      numSlots,
		slotDataSize:  slotDataSize,
		totalSlotSize: SlotHeaderSize + slotDataSize,
		headerOffset:  ctrlSize,
	}
	// 检查是否已初始化（例如写索引非零）
	writeIdx := binary.LittleEndian.Uint32(shm[offWriteIdx:])
	if writeIdx == 0 && shm[offWriteIdx+4] == 0 && shm[offWriteIdx+8] == 0 {
		// 首次初始化
		binary.LittleEndian.PutUint32(shm[offWriteIdx:], 0)
		binary.LittleEndian.PutUint32(shm[offReadIdx:], 0)
		shm[offTermFlag] = 0
	}
	return rb
}

// GetWriteSlot 获取下一个可写的槽位索引和数据起始指针
func (rb *RingBuffer) GetWriteSlot() (int, []byte, error) {
	writeIdx := binary.LittleEndian.Uint32(rb.mem[offWriteIdx:])
	readIdx := binary.LittleEndian.Uint32(rb.mem[offReadIdx:])

	// 计算下一个槽位
	nextWrite := (writeIdx + 1) % uint32(rb.numSlots)
	if nextWrite == readIdx {
		return -1, nil, errors.New("缓冲区已满")
	}

	// 计算槽位起始偏移
	slotStart := rb.headerOffset + int(writeIdx)*rb.totalSlotSize
	status := binary.LittleEndian.Uint32(rb.mem[slotStart:])
	if status != SlotStatusFree {
		return -1, nil, errors.New("当前槽位非空闲")
	}

	// 返回槽位索引和数据区指针（跳过头部）
	return int(writeIdx), rb.mem[slotStart+SlotHeaderSize : slotStart+rb.totalSlotSize], nil
}

// CommitWrite 提交写入的数据，设置状态和长度，并移动写指针
func (rb *RingBuffer) CommitWrite(slotIdx int, dataLen int, isTerm bool) {
	slotStart := rb.headerOffset + slotIdx*rb.totalSlotSize
	// 设置数据长度
	binary.LittleEndian.PutUint32(rb.mem[slotStart+4:], uint32(dataLen))
	// 设置状态
	status := SlotStatusReady
	if isTerm {
		status = SlotStatusTerm
	}
	binary.LittleEndian.PutUint32(rb.mem[slotStart:], uint32(status))

	// 移动写指针
	writeIdx := binary.LittleEndian.Uint32(rb.mem[offWriteIdx:])
	writeIdx = (writeIdx + 1) % uint32(rb.numSlots)
	binary.LittleEndian.PutUint32(rb.mem[offWriteIdx:], writeIdx)
}
