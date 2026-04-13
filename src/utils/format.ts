export const getTruncatedText = (text: string, maxLength: number = 150) => {
  if (text.startsWith("data:image")) return "🖼️ [图片数据]";
  if (text.length <= maxLength) return text;
  return text.substring(0, maxLength) + "...";
};

export const formatTime = (d: string) => {
  const diff = (Date.now()-new Date(d+"Z").getTime())/60000;
  if(diff<1)return"刚刚";
  if(diff<60)return`${Math.floor(diff)}分钟前`;
  const dt=new Date(d+"Z");
  return`${dt.getHours()}:${dt.getMinutes().toString().padStart(2,'0')}`;
};
