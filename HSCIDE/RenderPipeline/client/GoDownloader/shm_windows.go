package main

import (
	"fmt"
	"syscall"
	"unsafe"
)

// SharedMemory Windows 共享内存对象
type SharedMemory struct {
	name string
	size int
	hMap syscall.Handle // 文件映射句柄
	data []byte         // 映射后的切片
}

// CreateOrOpenSharedMemory 创建或打开共享内存对象
func CreateOrOpenSharedMemory(name string, size int) (*SharedMemory, error) {
	// 转换名称为 UTF16
	namePtr, err := syscall.UTF16PtrFromString(name)
	if err != nil {
		return nil, fmt.Errorf("UTF16转换失败：%v", err)
	}

	// 创建文件映射对象
	// 参数：句柄（0xFFFFFFFF 表示系统分页文件），安全属性，保护标志，大小高32位，大小低32位，名称
	hMap, err := syscall.CreateFileMapping(
		syscall.InvalidHandle,  // 使用系统分页文件
		nil,                    // 默认安全属性
		syscall.PAGE_READWRITE, // 可读写
		0,                      // 大小高32位
		uint32(size),           // 大小低32位
		namePtr,                // 名称
	)
	if err != nil {
		return nil, fmt.Errorf("创建文件映射对象失败：%v", err)
	}

	// 映射视图到进程地址空间
	addr, err := syscall.MapViewOfFile(
		hMap,
		syscall.FILE_MAP_WRITE, // 可写权限
		0, 0, 0,                // 从偏移0开始，映射整个文件
	)
	if err != nil {
		syscall.CloseHandle(hMap)
		return nil, fmt.Errorf("映射视图失败：%v", err)
	}

	// 转换为切片
	var data []byte
	sliceHeader := (*struct {
		addr uintptr
		len  int
		cap  int
	})(unsafe.Pointer(&data))
	sliceHeader.addr = addr
	sliceHeader.len = size
	sliceHeader.cap = size

	return &SharedMemory{
		name: name,
		size: size,
		hMap: hMap,
		data: data,
	}, nil
}

// Close 关闭共享内存
func (sm *SharedMemory) Close() error {
	if err := syscall.UnmapViewOfFile(uintptr(unsafe.Pointer(&sm.data[0]))); err != nil {
		return err
	}
	return syscall.CloseHandle(sm.hMap)
}

// Data 返回共享内存切片
func (sm *SharedMemory) Data() []byte {
	return sm.data
}
