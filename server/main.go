package main

import (
	"fmt"
	"net"

	"nexustui-server/Database"
	"nexustui-server/Handler"
	"nexustui-server/Hub"
)

func main() {
	fmt.Println("🚀 Starting NexusTUI Go Server...")

	// 1. Database (pgx/v5 pgxpool)
	if err := Database.InitDb(); err != nil {
		fmt.Printf("⚠️  Database warning: %v\n", err)
	}

	// 2. Start Hub and Handler
	serverHub := Hub.NewHub()
	tcpHandler := Handler.NewTCPHandler(serverHub)

	// 3. Bind to Port 9000 TCP Socket
	listener, err := net.Listen("tcp", "0.0.0.0:9000")
	if err != nil {
		panic(fmt.Sprintf("Failed to bind port 9000: %v", err))
	}
	defer listener.Close()

	fmt.Println("🟢 NexusTUI Hub LISTENING ON PORT 9000!")
	fmt.Println("   To connect: nc 192.168.1.11 9000")

	// 4. Sonsuz dongude baglantilari karsila
	for {
		conn, err := listener.Accept()
		if err != nil {
			continue
		}
		go tcpHandler.Handle(conn)
	}
}
