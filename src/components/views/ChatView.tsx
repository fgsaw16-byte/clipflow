import { useState, useRef, useEffect } from "react";
import { QRCodeSVG } from "qrcode.react";
import { Icon } from "../../utils/Icon";
import { ChatMessage } from "../../types";
import { sendToPhone, getLocalIp } from "../../api/sync";

interface ChatViewProps {
  isConnected: boolean;
  setIsConnected: React.Dispatch<React.SetStateAction<boolean>>;
  chatMessages: ChatMessage[];
  setChatMessages: React.Dispatch<React.SetStateAction<ChatMessage[]>>;
  settings: any;
}

export function ChatView({ isConnected, setIsConnected, chatMessages, setChatMessages, settings }: ChatViewProps) {
  const [ipAddress, setIpAddress] = useState("127.0.0.1");
  const [chatInput, setChatInput] = useState("");
  const chatEndRef = useRef<HTMLDivElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    async function loadIp() { setIpAddress(await getLocalIp()); }
    loadIp();
  }, []);

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [chatMessages]);

  const handlePcSend = async () => {
    if (!chatInput.trim()) return;
    const t = chatInput;
    await sendToPhone(t);
    setChatMessages(p => [...p, { text: t, isMe: true }]);
    setChatInput("");
  };

  const handleImageUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = async (event) => {
      const base64 = event.target?.result as string;
      if (base64) {
        await sendToPhone(base64);
        setChatMessages(p => [...p, { text: base64, isMe: true }]);
      }
    };
    reader.readAsDataURL(file);
    e.target.value = '';
  };

  const handleChatPaste = (e: React.ClipboardEvent) => {
    const items = e.clipboardData.items;
    for (let i = 0; i < items.length; i++) {
      if (items[i].type.indexOf("image") !== -1) {
        e.preventDefault();
        const blob = items[i].getAsFile();
        if (blob) {
          const reader = new FileReader();
          reader.onload = async (event) => {
            const base64 = event.target?.result as string;
            if (base64) {
              await sendToPhone(base64);
              setChatMessages(p => [...p, { text: base64, isMe: true }]);
            }
          };
          reader.readAsDataURL(blob);
        }
        return;
      }
    }
  };

  const handleChatKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handlePcSend(); }
  };

  return (
    <div key="chat" className="phone-connect-panel">
      {!isConnected ? (
        <>
          <div className="qr-wrapper"><QRCodeSVG value={`http://${ipAddress}:${settings.server_port || 19527}`} size={130} /></div>
          <h3>扫码进入聊天</h3>
          <p>确保在同一 Wi-Fi 下</p>
          <div className="ip-info">{ipAddress}:{settings.server_port || 19527}</div>
        </>
      ) : (
        <div className="chat-window">
          <div className="chat-header">🟢 已连接 <span className="disconnect-btn" onClick={() => {setIsConnected(false); setChatMessages([])}}><Icon name="refresh" size={16} /></span></div>
          <div className="chat-messages">{chatMessages.map((msg, idx) => (<div key={idx} className={`chat-bubble-row ${msg.isMe ? 'me' : 'other'}`}><div className="chat-bubble">{msg.text.startsWith('data:image') ? <img src={msg.text} className="chat-img"/> : msg.text}</div></div>))}<div ref={chatEndRef} /></div>
          <div className="chat-input-bar"><div className="chat-icon-btn" onClick={() => fileInputRef.current?.click()}><Icon name="plus-img" size={20} /></div><input type="file" accept="image/*" style={{display: 'none'}} ref={fileInputRef} onChange={handleImageUpload} /><input type="text" className="chat-input" value={chatInput} onChange={(e) => setChatInput(e.target.value)} onKeyDown={handleChatKeyDown} onPaste={handleChatPaste} /><button className="chat-send-btn" onClick={handlePcSend}>发送</button></div>
        </div>
      )}
    </div>
  );
}
