package main

import (
	"fmt"
	"syscall"
	"unsafe"
)

const EVENT_ALL_ACCESS = 0x1F0003

var (
	kernel32 = syscall.NewLazyDLL("kernel32.dll")

	procCreateEventW = kernel32.NewProc("CreateEventW")
	procOpenEventW   = kernel32.NewProc("OpenEventW")
	procSetEvent     = kernel32.NewProc("SetEvent")
	procResetEvent   = kernel32.NewProc("ResetEvent")
)

func CreateEvent(name string, manualReset bool) (syscall.Handle, error) {
	namePtr, err := syscall.UTF16PtrFromString(name)
	if err != nil {
		return 0, fmt.Errorf("UTF16 转换失败：%v", err)
	}

	var manualResetFlag uint32 = 0
	if manualReset {
		manualResetFlag = 1
	}

	ret, _, err := procCreateEventW.Call(
		0,                                // LPSECURITY_ATTRIBUTES (nil)
		uintptr(manualResetFlag),         // bManualReset
		0,                                // bInitialState (初始未触发)
		uintptr(unsafe.Pointer(namePtr)), // lpName
	)

	if ret == 0 {
		return 0, fmt.Errorf("创建事件失败：%v", err)
	}

	return syscall.Handle(ret), nil
}

func OpenEvent(name string) (syscall.Handle, error) {
	namePtr, err := syscall.UTF16PtrFromString(name)
	if err != nil {
		return 0, fmt.Errorf("UTF16 转换失败：%v", err)
	}

	ret, _, err := procOpenEventW.Call(
		uintptr(EVENT_ALL_ACCESS),        // dwDesiredAccess
		0,                                // bInheritHandle
		uintptr(unsafe.Pointer(namePtr)), // lpName
	)

	if ret == 0 {
		return 0, fmt.Errorf("打开事件失败：%v", err)
	}

	return syscall.Handle(ret), nil
}

func SetEvent(h syscall.Handle) error {
	ret, _, err := procSetEvent.Call(uintptr(h))
	if ret == 0 {
		return fmt.Errorf("设置事件失败：%v", err)
	}
	return nil
}

func ResetEvent(h syscall.Handle) error {
	ret, _, err := procResetEvent.Call(uintptr(h))
	if ret == 0 {
		return fmt.Errorf("重置事件失败：%v", err)
	}
	return nil
}

func WaitForSingleObject(h syscall.Handle, timeoutMs uint32) (uint32, error) {
	ret, _, err := syscall.NewLazyDLL("kernel32.dll").
		NewProc("WaitForSingleObject").
		Call(uintptr(h), uintptr(timeoutMs))

	if ret == 0xFFFFFFFF {
		return 0xFFFFFFFF, fmt.Errorf("等待失败：%v", err)
	}
	return uint32(ret), nil
}
