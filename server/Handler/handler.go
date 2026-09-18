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

	// Cihaz adini belirle
	clientName := fmt.Sprintf("Cihaz-%s", ip)
	if ip == "127.0.0.1" {
		clientName = "NexusTUI-PC"
	}

	client := &Hub.Client{
		Conn: conn,
		Ip:   ip,
		Name: clientName,
	}

	// Hub'a ekle ve sisteme duyur
	th.Hub.AddClient(client)
	go func() {
		if err := Database.SaveDevice(ip, clientName); err != nil {
			fmt.Printf("⚠️  Cihaz kayit hatasi: %v\n", err)
		}
	}()

	fmt.Fprintf(conn, "⚡ NexusTUI Ev Hub'ina Hos Geldin! Your Client Id: %s\n", clientName)
	th.Hub.BroadcastSystem(fmt.Sprintf("%s Client added.....", clientName))

	// Veritabanindan gecmis son 5 mesaji getir
	if msgs, err := Database.GetRecentMessages(5); err == nil && len(msgs) > 0 {
		fmt.Fprintln(conn, "--- Son Mesajlar ---")
		for _, m := range msgs {
			fmt.Fprintf(conn, "[%s] %s: %s\n", m.CreatedAt.Format("15:04"), m.Sender, m.Content)
		}
		fmt.Fprintln(conn, "--------------------")
	}

	// Gelen satirlari dinle
	scanner := bufio.NewScanner(conn)
	for scanner.Scan() {
		text := strings.TrimSpace(scanner.Text())
		if text == "" {
			continue
		}

		if text == "/quit" || text == "exit" {
			break
		}

		// Isim degistirme
		if strings.HasPrefix(text, "/nick ") {
			newName := strings.TrimPrefix(text, "/nick ")
			oldName := client.Name
			client.Name = newName
			go func() {
				_ = Database.SaveDevice(ip, newName)
			}()
			th.Hub.BroadcastSystem(fmt.Sprintf("%s ismini '%s' yapti!", oldName, newName))
			continue
		}

		// Pano (Clipboard) senkronizasyonu
		if strings.HasPrefix(text, "/clip ") {
			clipData := strings.TrimPrefix(text, "/clip ")
			go func() {
				_ = Database.SaveMessage(client.Name, "[PANO]: "+clipData)
			}()
			th.Hub.BroadcastSystem(fmt.Sprintf("📋 PANO GUNCEL (%s): %s", client.Name, clipData))
			continue
		}

		// Normal mesaj
		go func() {
			if err := Database.SaveMessage(client.Name, text); err != nil {
				fmt.Printf("⚠️  Mesaj kayit hatasi: %v\n", err)
			}
		}()
		fmt.Printf("📨 [%s]: %s\n", client.Name, text)
		th.Hub.Broadcast(client, text)
	}

	// Ayrilma islemi
	th.Hub.RemoveClient(conn)
	th.Hub.BroadcastSystem(fmt.Sprintf("%s ayrildi.", client.Name))
	fmt.Printf("❌ %s ayrildi.\n", client.Name)
}
