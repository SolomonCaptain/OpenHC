package main

import (
	"fmt"
	"syscall"
)

func CreateEvent(name string, manualReset bool) (syscall.Handle, error) {
	namePtr, err := syscall.UTF16PtrFromString(name)
	if err != nil {
		return 0, fmt.Errorf("UTF16转换失败：%v", err)
	}
	return syscall.CreateEvent(nil, 1, 0, namePtr) // 参数：安全属性，manualReset，初始状态，名称
}

func OpenEvent(name string) (syscall.Handle, error) {
	namePtr, err := syscall.UTF16PtrFromString(name)
	if err != nil {
		return 0, fmt.Errorf("UTF16转换失败：%v", err)
	}
	return syscall.OpenEvent(syscall.EVENT_ALL_ACCESS, false, namePtr)
}

func SetEvent(h syscall.Handle) error {
	return syscall.SetEvent(h)
}

func ResetEvent(h syscall.Handle) error {
	return syscall.ResetEvent(h)
}

func WaitForSingleObject(h syscall.Handle, timeoutMs uint32) (uint32, error) {
	// 返回值为0表示成功，其他表示超时或错误
	return syscall.WaitForSingleObject(h, timeoutMs)
}
