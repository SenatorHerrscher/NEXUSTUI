package Handler

import (
	"bufio"
	"fmt"
	"net"
	"strings"

	"nexustui-server/Database"
	"nexustui-server/Hub"
)

type TCPHandler struct {
	Hub *Hub.Hub
}

func NewTCPHandler(h *Hub.Hub) *TCPHandler {
	return &TCPHandler{Hub: h}
}

func (th *TCPHandler) Handle(conn net.Conn) {
	defer conn.Close()

	remoteAddr := conn.RemoteAddr().String()
	ip := strings.Split(remoteAddr, ":")[0]

	// Determine client name
	clientName := fmt.Sprintf("Device-%s", ip)
	if ip == "127.0.0.1" {
		clientName = "NexusTUI-PC"
	}

	client := &Hub.Client{
		Conn: conn,
		Ip:   ip,
		Name: clientName,
	}

	// Add to hub and announce
	th.Hub.AddClient(client)
	go func() {
		if err := Database.SaveDevice(ip, clientName); err != nil {
			fmt.Printf("⚠️  Device registration error: %v\n", err)
		}
	}()

	fmt.Fprintf(conn, "⚡ Welcome to NexusTUI LAN Hub! Client ID: %s\n", clientName)
	th.Hub.BroadcastSystem(fmt.Sprintf("%s joined the network.", clientName))

	// Fetch recent 5 messages from database
	if msgs, err := Database.GetRecentMessages(5); err == nil && len(msgs) > 0 {
		fmt.Fprintln(conn, "--- Recent Messages ---")
		for _, m := range msgs {
			fmt.Fprintf(conn, "[%s] %s: %s\n", m.CreatedAt.Format("15:04"), m.Sender, m.Content)
		}
		fmt.Fprintln(conn, "-----------------------")
	}

	// Listen for incoming lines
	scanner := bufio.NewScanner(conn)
	for scanner.Scan() {
		text := strings.TrimSpace(scanner.Text())
		if text == "" {
			continue
		}

		if text == "/quit" || text == "exit" {
			break
		}

		// Nickname change
		if strings.HasPrefix(text, "/nick ") {
			newName := strings.TrimPrefix(text, "/nick ")
			oldName := client.Name
			client.Name = newName
			go func() {
				_ = Database.SaveDevice(ip, newName)
			}()
			th.Hub.BroadcastSystem(fmt.Sprintf("%s changed name to '%s'", oldName, newName))
			continue
		}

		// Clipboard synchronization
		if strings.HasPrefix(text, "/clip ") {
			clipData := strings.TrimPrefix(text, "/clip ")
			go func() {
				_ = Database.SaveMessage(client.Name, "[CLIPBOARD]: "+clipData)
			}()
			th.Hub.BroadcastSystem(fmt.Sprintf("📋 CLIPBOARD UPDATED (%s): %s", client.Name, clipData))
			continue
		}

		// Regular message
		go func() {
			if err := Database.SaveMessage(client.Name, text); err != nil {
				fmt.Printf("⚠️  Message save error: %v\n", err)
			}
		}()
		fmt.Printf("📨 [%s]: %s\n", client.Name, text)
		th.Hub.Broadcast(client, text)
	}

	// Disconnection cleanup
	th.Hub.RemoveClient(conn)
	th.Hub.BroadcastSystem(fmt.Sprintf("%s disconnected.", client.Name))
	fmt.Printf("❌ %s disconnected.\n", client.Name)
}
