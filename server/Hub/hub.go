package Hub

import (
	"fmt"
	"net"
	"sync"
)

type Client struct {
	Conn net.Conn
	Ip   string
	Name string
}
type Hub struct {
	Clients map[net.Conn]*Client
	mutex   sync.RWMutex
}

func NewHub() *Hub {
	return &Hub{
		Clients: make(map[net.Conn]*Client),
	}
}

func (h *Hub) AddClient(Client *Client) {
	h.mutex.Lock()
	defer h.mutex.Unlock()
	h.Clients[Client.Conn] = Client
}

func (h *Hub) RemoveClient(conn net.Conn) *Client {
	h.mutex.Lock()
	defer h.mutex.Unlock()
	client, exists := h.Clients[conn]
	if exists {
		delete(h.Clients, conn)
	}
	return client
}

func (h *Hub) Broadcast(sender *Client, message string) {
	h.mutex.RLock()
	defer h.mutex.RUnlock()

	for conn := range h.Clients {
		if conn != sender.Conn {
			fmt.Fprintf(conn, "[%s]: %s\n", sender.Name, message)
		}
	}
}

func (h *Hub) BroadcastSystem(message string) {
	h.mutex.RLock()
	defer h.mutex.RUnlock()

	for conn := range h.Clients {
		fmt.Fprintf(conn, "[SYSTEM]: %s\n", message)
	}
}
