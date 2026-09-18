package main

import (
	"fmt"
	"net"

	"nexustui-server/Database"
	"nexustui-server/Handler"
	"nexustui-server/Hub"
)

func main() {
	fmt.Println("🚀 NexusTUI Go Sunucusu Baslatiliyor...")

	// 1. Veritabani (pgx/v5 pgxpool)
	if err := Database.InitDb(); err != nil {
		fmt.Printf("⚠️  Veritabani uyarisi: %v\n", err)
	}

	// 2. Hub ve Handler baslat
	serverHub := Hub.NewHub()
	tcpHandler := Handler.NewTCPHandler(serverHub)

	// 3. Port 9000 TCP Soketini ac
	listener, err := net.Listen("tcp", "0.0.0.0:9000")
	if err != nil {
		panic(fmt.Sprintf("Port 9000 dinlenemedi: %v", err))
	}
	defer listener.Close()

	fmt.Println("🟢 NexusTUI Santrali PORT 9000'de DINLEMEDE!")
	fmt.Println("   Baglanmak icin: nc 192.168.1.11 9000")

	// 4. Sonsuz dongude baglantilari karsila
	for {
		conn, err := listener.Accept()
		if err != nil {
			continue
		}
		go tcpHandler.Handle(conn)
	}
}
